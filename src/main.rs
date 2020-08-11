//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

extern crate pretty_env_logger;
extern crate tokio;
extern crate chrono;
extern crate headers;

#[macro_use]
extern crate lazy_static;

use std::sync::Arc;
use std::collections::HashMap;
use std::result::Result;
use std::net::SocketAddr;
use std::ops::Bound;

use tokio::signal;
use tokio::sync;
use tokio::time;
use tokio::task;
use tokio::join;
use tokio::fs::File;
use tokio_util::codec::{ BytesCodec, FramedRead };

use futures::{ StreamExt, SinkExt };

use warp::{ Filter, http::HeaderMap, filters, reply::Reply, reply::Response, reject::Rejection, hyper::Body };

use headers::HeaderMapExt;

use warp::http::header;
use warp::http::StatusCode;
use warp::ws::{ Message, WebSocket };

use serde::{ Serialize, Deserialize, de::DeserializeOwned };

mod context;
mod mpdcom;
mod mpdfifo;
mod event;
mod asyncread;
mod bt;
mod btctrl;
mod iolist;
mod albumart;

use crate::asyncread::GetWakeShutdownFlag;
use crate::asyncread::GetMimeType;

///
type StrResult = Result< String, Rejection >;

///
type RespResult = Result< Response, Rejection >;

///
fn json_response< T: ?Sized + Serialize >( t : &T ) -> Response
{
    let mut r = Response::new(
        match serde_json::to_string( t )
        {
            Ok( x ) => { x }
        ,   _       => { String::new() }
        }.into()
    );
    r.headers_mut().insert( header::CONTENT_TYPE, header::HeaderValue::from_str( &mime::APPLICATION_JSON.to_string() ).unwrap() );
    r
}

///
fn internal_server_error( t : &str ) -> Response
{
    let mut r = Response::new( String::from( t ).into() );
    *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    r
}

///
fn bad_request( t : &str ) -> Response
{
    let mut r = Response::new( String::from( t ).into() );
    *r.status_mut() = StatusCode::BAD_REQUEST;
    r
}

async fn album_art_response( arwlctx : context::ARWLContext, headers: HeaderMap, path : String ) -> RespResult
{
    if !arwlctx.read().await.shutdown
    {
        let path =
            path.split( '/' )
                .skip( 2 )
                .map( |x| x.to_string() )
                .collect::< Vec< String > >()
                .join( "/" )
                ;

        match albumart::get_albumart( arwlctx.clone(), &path ).await
        {
            albumart::AlbumartResult::BadRequest =>
            {
                return Ok( bad_request( "Invalid path" ) )
            }

        ,   albumart::AlbumartResult::Binary( mime, bytes ) =>
            {
            }

        ,   _ => { /* nop */ }
        };
    }

    Err( warp::reject::not_found() )
}

async fn proxy_stream_response( arwlctx : context::ARWLContext, _headers: HeaderMap ) -> RespResult
{
    if !arwlctx.read().await.shutdown
    {
        let url = String::from( &arwlctx.read().await.config.mpd_httpd_url );

        if url.as_str() != ""
        {
            match asyncread::HttpRedirect::new( &url ).await
            {
                Ok( hr ) =>
                {
                    arwlctx.write().await.sdf_add( hr.get_wake_shutdown_flag() );

                    let status = hr.get_response().status();
                    let headers = hr.get_response().headers().clone();

                    let stream = FramedRead::new( hr, BytesCodec::new() );
                    let body = Body::wrap_stream( stream );
                    let mut resp = Response::new( body );

                    *resp.status_mut() = status;
                    resp.headers_mut().extend( headers );

                    return Ok( resp );
                }
            ,   Err( x ) =>
                {
                    log::error!( "proxy_stream_response error. {:?}", x );
                }
            }
        }
    }

    Err( warp::reject::not_found() )
}

///
#[derive(Debug, Deserialize, Clone)]
struct AsoundParam
{
    a_rate      : Option<u32>
,   a_channels  : Option<u8>
,   a_buffer_t  : Option<u32>
,   a_period_t  : Option<u32>
/*
,   lm_brate    : Option<u32>
,   lm_a_brate  : Option<u32>
*/
}

async fn asound_response( arwlctx : context::ARWLContext, _headers: HeaderMap, dev : String, param : AsoundParam  ) -> RespResult
{
    if !arwlctx.read().await.shutdown
    {
        let aclep = asyncread::AlsaCaptureEncodeParam
        {
            a_rate      : param.a_rate
        ,   a_channels  : param.a_channels
        ,   a_buffer_t  : param.a_buffer_t
        ,   a_period_t  : param.a_period_t
        ,   lm_brate    : None
        ,   lm_a_brate  : None
        };

        let use_lame = false;

        if use_lame
        {
            #[ cfg( feature = "lame-sys" ) ]
            match asyncread::AlsaCaptureLameEncode::new( dev, aclep )
            {
                Ok( acle ) =>
                {
                    let mime_type = acle.get_mime_type();

                    arwlctx.write().await.sdf_add( acle.get_wake_shutdown_flag() );

                    let stream = FramedRead::new( acle, BytesCodec::new() );
                    let body = Body::wrap_stream( stream );
                    let mut resp = Response::new( body );

                    resp.headers_mut().typed_insert( headers::ContentType::from( mime_type ) );
                    resp.headers_mut().typed_insert( headers::AcceptRanges::bytes() );
                    resp.headers_mut().typed_insert( headers::Pragma::no_cache() );
                    resp.headers_mut().typed_insert( headers::CacheControl::new().with_no_store().with_no_cache() );

                    return Ok( resp );
                }
            ,   Err( x ) =>
                {
                    log::error!( "asound_response error. {:?}", x );
                }
            }
        }
        else
        {
            match asyncread::AlsaCaptureFlacEncode::new( dev, aclep )
            {
                Ok( acfe ) =>
                {
                    let mime_type = acfe.get_mime_type();

                    arwlctx.write().await.sdf_add( acfe.get_wake_shutdown_flag() );

                    let stream = FramedRead::new( acfe, BytesCodec::new() );
                    let body = Body::wrap_stream( stream );
                    let mut resp = Response::new( body );

                    resp.headers_mut().typed_insert( headers::ContentType::from( mime_type ) );
                    resp.headers_mut().typed_insert( headers::AcceptRanges::bytes() );
                    resp.headers_mut().typed_insert( headers::Pragma::no_cache() );
                    resp.headers_mut().typed_insert( headers::CacheControl::new().with_no_store().with_no_cache() );

                    return Ok( resp );
                }
            ,   Err( x ) =>
                {
                    log::error!( "asound_response error. {:?}", x );
                }
            }
        }
    }

    Err( warp::reject::not_found() )
}

///
async fn make_file_response( arwlctx : context::ARWLContext, headers: HeaderMap, path: &std::path::Path ) -> RespResult
{
    if log::log_enabled!( log::Level::Debug )
    {
        if let Some( ua ) = headers.typed_get::< headers::UserAgent >()
        {
            lazy_static!
            {
                static ref RE : regex::Regex =
                    regex::Regex::new( context::MPD_USER_AGENT ).unwrap();
            }

            if RE.is_match( ua.as_str() )
            {
                log::debug!( "{:?}", &headers );
            }
        }
    }

    match File::open( path ).await
    {
        Ok( file ) =>
        {
            let metadata = file.metadata().await;

            if let Ok( metadata ) = metadata
            {
                let max_len = metadata.len();

                if let Some( range ) = headers.typed_get::<headers::Range>()
                {
                    if let Some( ( st, ed ) ) = range.iter().next()
                    {
                        let st = match st
                        {
                            Bound::Unbounded => 0,
                            Bound::Included(s) => s,
                            Bound::Excluded(s) => s + 1,
                        };

                        let ed = match ed
                        {
                            Bound::Unbounded => max_len,
                            Bound::Included(s) => s + 1,
                            Bound::Excluded(s) => s,
                        };

                        if st < ed && ed <= max_len
                        {
                            match asyncread::FileRangeRead::new( file, st, ed ).await
                            {
                                Ok( filerange ) =>
                                {
                                    arwlctx.write().await.sdf_add( filerange.get_wake_shutdown_flag() );

                                    let len = filerange.len();

                                    let stream = FramedRead::new( filerange, BytesCodec::new() );

                                    let body = Body::wrap_stream( stream );
                                    let mut resp = Response::new( body );

                                    let mime = mime_guess::from_path( path ).first_or_octet_stream();

                                    resp.headers_mut().typed_insert( headers::ContentLength( len ) );
                                    resp.headers_mut().typed_insert( headers::ContentType::from( mime ) );
                                    resp.headers_mut().typed_insert( headers::AcceptRanges::bytes() );

                                    *resp.status_mut() = StatusCode::PARTIAL_CONTENT;
                                    resp.headers_mut().typed_insert(
                                        headers::ContentRange::bytes( st..ed, len ).expect( "valid ContentRange" )
                                    );

                                    return Ok( resp );
                                }
                            ,   Err( x ) =>
                                {
                                    log::error!( "{:?}", x );
                                }
                            }
                        }

                        let mut resp = Response::new(Body::empty());
                        *resp.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
                        resp.headers_mut().typed_insert( headers::ContentRange::unsatisfied_bytes( max_len ) );

                        return Ok( resp );
                    }
                }
                else
                {
                    match asyncread::FileRangeRead::new( file, 0, max_len ).await
                    {
                        Ok( filerange ) =>
                        {
                            arwlctx.write().await.sdf_add( filerange.get_wake_shutdown_flag() );

                            let stream = FramedRead::new( filerange, BytesCodec::new() );
                            let body = Body::wrap_stream( stream );
                            let mut resp = Response::new( body );

                            let mime = mime_guess::from_path( path ).first_or_octet_stream();

                            resp.headers_mut().typed_insert( headers::ContentLength( max_len ) );
                            resp.headers_mut().typed_insert( headers::ContentType::from( mime ) );
                            resp.headers_mut().typed_insert( headers::AcceptRanges::bytes() );

                            return Ok( resp );
                        }
                    ,   Err( x ) =>
                        {
                            log::error!( "{:?}", x );
                        }
                    }
                }
            }
        }
    ,   Err( x ) =>
        {
            log::error!( "{:?}", x );
        }
    }

    Err( warp::reject::not_found() )
}

///
fn check_path( path : &str )
    -> Result< String, Rejection >
{
    let mut p = Vec::< String >::new();

    for x in path.split( '/' )
    {
        match x
        {
            "\\"        => { return Err( warp::reject::not_found() ); }
        ,   "" | "."    => {}
        ,   ".."        =>
            {
                if p.pop().is_none()
                {
                    return Err( warp::reject::not_found() );
                }
            }
        ,   _           => { p.push( String::from( x ) ); }
        }
    }

    Ok( p.join( "/" ) )
}

///
enum FileResponse
{
    Main
,   Favicon
,   Common( String )
,   Theme( String )
,   Tsound( String )
}

///
async fn theme_file_response( arwlctx : context::ARWLContext, headers: HeaderMap, target_path : FileResponse ) -> RespResult
{
    let mut path_base =
        match target_path
        {
            FileResponse::Main | FileResponse::Favicon | FileResponse::Theme(_) =>
            {
                arwlctx.read().await.get_theme_path()
            }
        ,   FileResponse::Common(_) => { arwlctx.read().await.get_common_path() }
        ,   FileResponse::Tsound(_) => { arwlctx.read().await.get_tsound_path()  }
        };

    let do_unshift =
        match target_path
        {
            FileResponse::Main | FileResponse::Favicon  => { false }
        ,   _                                           => { true }
        };

    let path =
        match target_path
        {
            FileResponse::Main      => { String::from( context::THEME_MAIN ) }
        ,   FileResponse::Favicon   => { String::from( context::THEME_FAVICON_ICO ) }
        ,   FileResponse::Common(x) => { x }
        ,   FileResponse::Theme(x)  => { x }
        ,   FileResponse::Tsound(x) => { x }
        };

    let path =
        {
            if do_unshift
            {
                path.split( '/' )
                    .skip( 2 )
                    .map( |x| x.to_string() )
                    .collect::< Vec< String > >()
                    .join( "/" )
            }
            else
            {
                path
            }
        };

    match check_path( &path )
    {
        Err( x ) => { RespResult::Err( x ) }
    ,   Ok( path ) =>
        {
            path_base.push( &path );
            make_file_response( arwlctx, headers, &path_base ).await
        }
    }
}

///
#[derive(Debug, Deserialize, Clone)]
struct CmdParam
{
    cmd  : String
,   arg1 : Option<String>
,   arg2 : Option<String>
,   arg3 : Option<String>
}

///
impl CmdParam
{
    fn to_request( &self ) -> ( mpdcom::MpdComRequest, sync::oneshot::Receiver< mpdcom::MpdComResult > )
    {
        let mut cmd = self.cmd.trim_end().to_lowercase();

        let reqval =
            match cmd.as_str()
            {
                "setvol" =>
                {
                    if self.arg1.is_some()
                    {
                        mpdcom::MpdComRequestType::SetVol( String::from( self.arg1.as_ref().unwrap().as_str() ) )
                    }
                    else
                    {
                        mpdcom::MpdComRequestType::Nop
                    }
                }

            ,   "setmute" =>
                {
                    if self.arg1.is_some()
                    {
                        mpdcom::MpdComRequestType::SetMute( String::from( self.arg1.as_ref().unwrap().as_str() ) )
                    }
                    else
                    {
                        mpdcom::MpdComRequestType::Nop
                    }
                }

            ,   "addurl" =>
                {
                    if self.arg1.is_some()
                    {
                        let url = String::from( self.arg1.as_ref().unwrap().as_str() );

                        let arg =
                            if self.arg2.is_some()
                            {
                                String::from( self.arg2.as_ref().unwrap().as_str() )
                            }
                            else
                            {
                                String::new()
                            };

                        mpdcom::MpdComRequestType::AddUrl( url, arg )
                    }
                    else
                    {
                        mpdcom::MpdComRequestType::Nop
                    }
                }

            ,   "addauxin" =>
                {
                    if self.arg1.is_some()
                    {
                        let url = String::from( self.arg1.as_ref().unwrap().as_str() );

                        let arg =
                            if self.arg2.is_some()
                            {
                                String::from( self.arg2.as_ref().unwrap().as_str() )
                            }
                            else
                            {
                                String::new()
                            };

                        mpdcom::MpdComRequestType::AddAuxIn( url, arg )
                    }
                    else
                    {
                        mpdcom::MpdComRequestType::Nop
                    }
                }

            ,   "testsound" =>
                {
                    mpdcom::MpdComRequestType::TestSound
                }

            ,   "" =>
                {
                    mpdcom::MpdComRequestType::Nop
                }

            ,   _ =>
                {
                    if self.arg1.is_some()
                    {
                        if let Some( x ) = self.arg1.as_ref()
                        {
                            if x.trim() != ""
                            {
                                cmd += " ";
                                cmd += &mpdcom::quote_arg( &x );
                            }
                        }
                    }

                    if self.arg2.is_some()
                    {
                        if let Some( x ) = self.arg2.as_ref()
                        {
                            if x.trim() != ""
                            {
                                cmd += " ";
                                cmd += &mpdcom::quote_arg( &x );
                            }
                        }
                    }

                    if self.arg3.is_some()
                    {
                        if let Some( x ) = self.arg3.as_ref()
                        {
                            if x.trim() != ""
                            {
                                cmd += " ";
                                cmd += &mpdcom::quote_arg( &x );
                            }
                        }
                    }

                    mpdcom::MpdComRequestType::Cmd( cmd )
                }
            };

        let ( mut req, rx ) = mpdcom::MpdComRequest::new();

        req.req = reqval;

        ( req, rx )
    }
}

///
async fn cmd_response( arwlctx : context::ARWLContext, param : CmdParam ) -> RespResult
{
    log::debug!( "{:?}", &param );

    let ( req, rx ) = param.to_request();

    let _ = arwlctx.write().await.mpdcom_tx.send( req ).await;

    Ok(
        match rx.await
        {
            Ok( x )  => json_response( &x )
        ,   Err( x ) => internal_server_error( &format!( "{:?}", x ) )
        }
    )
}

///
async fn status_response( arwlctx : context::ARWLContext ) -> StrResult
{
    Ok( String::from( &arwlctx.read().await.mpd_status_json ) )
}

///
async fn spec_head_response( arwlctx : context::ARWLContext ) -> StrResult
{
    Ok( String::from( &arwlctx.read().await.spec_head_json ) )
}

///
async fn spec_data_response( arwlctx : context::ARWLContext ) -> StrResult
{
    Ok( String::from( &arwlctx.read().await.spec_data_json ) )
}

///
#[derive(Debug, Deserialize, Clone)]
struct ConfigParam
{
    update : Option< String >
}

///
async fn config_response( arwlctx : context::ARWLContext, param : ConfigParam ) -> RespResult
{
    if param.update.is_some()
    {
        let mut ctx = arwlctx.write().await;

        let newval = String::from( param.update.as_ref().unwrap().trim_end() );

        if newval != ""
        {
            if let Some( err ) = ctx.update_config_dyn( &newval )
            {
                return Ok( json_response( &context::ConfigDynOutputResult::Err( err ) ) )
            }
        }
    }

    let ctx = arwlctx.read().await;

    Ok( json_response( &context::ConfigDynOutputResult::Ok( ctx.make_config_dyn_output() ) ) )
}

///
async fn ws_response( arwlctx : context::ARWLContext, ws : WebSocket, addr: Option< SocketAddr > )
{
    let (
        ws_sess_stop
    ,   ws_no
    ,   ws_sig
    ,   mut ev_rx
    ,   ws_status_intv
    ,   ws_data_intv
    ,   ws_send_intv
    ,   mut last_mpd_status_json
    ,       last_spec_head_json
    ,   mut last_spec_data_json
    ,   mut last_bt_status_json
    ,   mut last_io_list_json
    ) =
    {
        let mut ctx = arwlctx.write().await;

        ctx.ws_sess_no += 1;

        let ws_no = ctx.ws_sess_no;
        let ws_sig = format!( "ws:{}:{:?}", ws_no, &addr );

        let ( ev_tx, ev_rx ) = event::make_channel();

        ctx.ws_sessions.insert( ws_no, context::WsSession{ ws_sig : String::from( &ws_sig ), ev_tx } );

        (
            ctx.ws_sess_stop
        ,   ws_no
        ,   ws_sig
        ,   ev_rx
        ,   ctx.ws_status_intv
        ,   ctx.ws_data_intv
        ,   ctx.ws_send_intv
        ,   String::from( &ctx.mpd_status_json )
        ,   String::from( &ctx.spec_head_json )
        ,   String::from( &ctx.spec_data_json )
        ,   String::from( &ctx.bt_status_json )
        ,   String::from( &ctx.io_list_json )
        )
    };

    log::debug!( "wss start. {:?}", &ws_sig );

    macro_rules! cleanup
    {
        () =>
        {
            log::debug!( "wss stop. {:?}", &ws_sig );

            let mut ctx = arwlctx.write().await;
            ctx.ws_sessions.remove( &ws_no );
        }
    };

    if ws_sess_stop
    {
        cleanup!();
        return;
    }

    let ( mut ws_tx, mut ws_rx ) = ws.split();

    for x in vec![ &last_mpd_status_json, &last_spec_head_json, &last_spec_data_json, &last_io_list_json ]
    {
        if let Err(x) = ws_tx.send( Message::text( x ) ).await
        {
            log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
            cleanup!();
            return;
        }
    }

    let ( mut ev_tx1, mut ev_rx1 ) = event::make_channel();

    let ws_sig_rx = format!( "{}:RX", &ws_sig );

    let h_rx = task::spawn( async move
        {
            loop
            {
                if event::event_shutdown( &mut ev_rx1 ).await
                {
                    break;
                }

                if let Ok( r ) =  time::timeout( event::EVENT_WAIT_TIMEOUT * 4, ws_rx.next() ).await
                {
                    if let Some( recv ) = r
                    {
                        match recv
                        {
                            Err( e ) =>
                            {
                                log::warn!( "web socket error. {:?} {:?}", &e, &ws_sig_rx );
                            }
                        ,   Ok( x ) =>
                            {
                                log::debug!( "web socket recv. {:?} {:?}", &x, &ws_sig_rx );
                            }
                        }
                    }
                }
            }

            log::debug!( "wss stop. {:?}", &ws_sig_rx );
        }
    );

    let mut last_check_mpd_status   = time::Instant::now();
    let mut last_send_mpd_status    = time::Instant::now();

    let mut last_check_data         = time::Instant::now();
    let mut last_send_data          = time::Instant::now();

    let mut last_check_bt_status    = time::Instant::now();
    let mut last_send_bt_status     = time::Instant::now();

    let mut last_check_io_list      = time::Instant::now();
    let mut last_send_io_list       = time::Instant::now();

    let mut last_send_head          = time::Instant::now();

    let mut last_bt_notice_json     = String::new();
    let mut last_check_bt_notice    = time::Instant::now();

    macro_rules! check_n_send
    {
        ( $intv:expr, $last_check:expr, $last_send:expr, $last_json:expr, $src_json:ident ) =>
        {
            if $last_check.elapsed() > $intv
            {
                $last_check = time::Instant::now();

                if
                {
                    let ctx = arwlctx.read().await;

                    if ctx.$src_json != $last_json
                    {
                        $last_json = String::from( &ctx.$src_json );
                        true
                    }
                    else
                    {
                        false
                    }
                } || $last_send.elapsed() > ws_send_intv
                {
                    $last_send = time::Instant::now();

                    if let Err(x) = ws_tx.send( Message::text( &$last_json ) ).await
                    {
                        log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
                        break;
                    }
                }
            }
        }
    }

    loop
    {
        if event::event_shutdown( &mut ev_rx ).await
        {
            break;
        }

        check_n_send!( ws_status_intv,  last_check_mpd_status,  last_send_mpd_status,   last_mpd_status_json,   mpd_status_json );
        check_n_send!( ws_data_intv,    last_check_data,        last_send_data,         last_spec_data_json,    spec_data_json );
        check_n_send!( ws_status_intv,  last_check_bt_status,   last_send_bt_status,    last_bt_status_json,    bt_status_json );
        check_n_send!( ws_status_intv,  last_check_io_list,     last_send_io_list,      last_io_list_json,      io_list_json );

        if last_send_head.elapsed() > ws_send_intv
        {
            last_send_head = time::Instant::now();

            if let Err(x) = ws_tx.send( Message::text( &last_bt_status_json ) ).await
            {
                log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
                break;
            }
        }

        if last_check_bt_notice.elapsed() > ws_status_intv
        {
            last_check_bt_notice = time::Instant::now();

            if
            {
                let ctx = arwlctx.read().await;

                if ctx.bt_notice_json != last_bt_notice_json
                {
                    last_bt_notice_json = String::from( &ctx.bt_notice_json );

                    if last_bt_notice_json != ""
                    {
                        true
                    }
                    else
                    {
                        false
                    }
                }
                else
                {
                    false
                }
            }
            {
                if let Err(x) = ws_tx.send( Message::text( &last_bt_notice_json ) ).await
                {
                    log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
                    break;
                }
            }
        }
    }

    let ( mut req, rx ) = event::new_request();
    req.req = event::EventRequestType::Shutdown;
    let _ = ev_tx1.send( req ).await;
    let _ = rx.await;
    let _ = join!( h_rx );

    cleanup!();
}

#[derive(Debug, Deserialize, Clone)]
struct BtCmdParam
{
    cmd : String
,   aid : String
,   did : String
,   sw  : bool
,   arg : Option< String >
}

///
impl BtCmdParam
{
    fn to_request( &self ) -> ( btctrl::BtctrlRequest, sync::oneshot::Receiver< btctrl::BtctrlResult > )
    {
        let cmd = self.cmd.trim_end().to_lowercase();

        let ( mut req, rx ) = btctrl::BtctrlRequest::new();

        req.req =
            btctrl::BtctrlRequestType::Cmd
            (
                cmd
            ,   String::from( &self.aid )
            ,   String::from( &self.did )
            ,   self.sw
            ,   if let Some( x ) = self.arg.as_ref() { Some( String::from( x ) ) } else { None }
            );

        ( req, rx )
    }
}

///
async fn bt_cmd_response( arwlctx : context::ARWLContext, param : BtCmdParam ) -> RespResult
{
    log::debug!( "{:?}", &param );

    let ( req, rx ) = param.to_request();

    let _ = arwlctx.write().await.btctrl_tx.send( req ).await;

    Ok(
        match rx.await
        {
            Ok( x )  => json_response( &x )
        ,   Err( x ) => internal_server_error( &format!( "{:?}", x ) )
        }
    )
}

#[derive(Debug, Deserialize, Clone)]
struct BtReplyParam
{
    reply_token : String
,   ok          : bool
}

///
async fn bt_reply_response( arwlctx : context::ARWLContext, param : BtReplyParam ) -> RespResult
{
    log::debug!( "{:?}", &param );

    let req = btctrl::BtctrlRepryType::Reply( String::from( &param.reply_token ), param.ok );

    let mut ctx = arwlctx.write().await;

    if ctx.bt_agent_io_rx_opend
    {
        let _ = ctx.bt_agent_io_tx.send( req ).await;
    }

    Ok( json_response( "{Ok:{}}" ) )
}

async fn io_list_response( arwlctx : context::ARWLContext ) -> RespResult
{
    Ok( json_response( &iolist::io_list_result( arwlctx ).await ) )
}

#[derive(Debug, Deserialize, Clone)]
struct SetOutputParam
{
    url : String
,   sw  : bool
}

///
async fn set_output_response( arwlctx : context::ARWLContext, param : SetOutputParam ) -> RespResult
{
    log::debug!( "{:?}", &param );

    lazy_static!
    {
        static ref RE_MPD : regex::Regex =
            regex::Regex::new( context::HIDAMARI_EXT_SINK_MPD_PROTO ).unwrap();

        static ref RE_ALSEA : regex::Regex =
            regex::Regex::new( context::HIDAMARI_EXT_SINK_ALSA_PROTO ).unwrap();
    }

    if let Some( cap ) = RE_MPD.captures( &param.url )
    {
        let id = cap.get( 1 ).unwrap().as_str();

        let ( mut req, rx ) = mpdcom::MpdComRequest::new();

        req.req = mpdcom::MpdComRequestType::Cmd
            (
                format!
                (
                    "{} {}"
                ,   if param.sw { "enableoutput" } else { "disableoutput" }
                ,   mpdcom::quote_arg( id )
                )
            );

        let _ = arwlctx.write().await.mpdcom_tx.send( req ).await;

        Ok(
            match rx.await
            {
                Ok( x )  => json_response( &x )
            ,   Err( x ) => internal_server_error( &format!( "{:?}", x ) )
            }
        )
    }
    else if let Some( cap ) = RE_ALSEA.captures( &param.url )
    {
        let tail = &param.url[ cap.get( 0 ).unwrap().end() .. ];

        let ( mut req, rx ) = mpdfifo::MpdfifoRequest::new();

        req.req = mpdfifo::MpdfifoRequestType::AlsaEnable( String::from( tail ), param.sw );

        let _ = arwlctx.write().await.mpdfifo_tx.send( req ).await;

        Ok(
            match rx.await
            {
                Ok( x )  => json_response( &x )
            ,   Err( x ) => internal_server_error( &format!( "{:?}", x ) )
            }
        )
    }
    else
    {
        Ok( bad_request( "Invalid url value" ) )
    }
}

///
async fn test_response( _arwlctx : context::ARWLContext, _param : HashMap< String, String > ) -> StrResult
{
    StrResult::Ok( String::new() )
}

///
fn make_route_getpost< T : DeserializeOwned + Send + 'static >()
    -> impl Filter< Extract = ( T, ), Error = Rejection > + Copy
{
    warp::get()
    .and(
        warp::query::< T >()
    )
    .or(
        warp::post()
        .and(
            warp::body::content_length_limit( 1024 * 32 )  // Limit the body to 32kb...
        )
        .and(
            warp::body::form::< T >()
        )
    )
    .unify()
}

///
async fn make_route( arwlctx : context::ARWLContext )
    -> filters::BoxedFilter< ( impl Reply, ) >
{
    let product = String::from( &arwlctx.read().await.product );
    let version = String::from( &arwlctx.read().await.version );

    let arwlctx_clone_filter = move ||
        {
            let x_arwlctx = arwlctx.clone();
            warp::any().map( move || x_arwlctx.clone() )
        };

    let r_root =
        warp::path::end()
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and_then( | arwlctx : context::ARWLContext, headers: HeaderMap | async move
            {
                theme_file_response( arwlctx, headers, FileResponse::Main ).await
            }
        );

    let r_favicon =
        warp::path!( "favicon.ico" )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and_then( | arwlctx : context::ARWLContext, headers: HeaderMap | async move
            {
                theme_file_response( arwlctx, headers, FileResponse::Favicon ).await
            }
        );

    let r_common =
        warp::path!( "common" / .. )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and( warp::path::full() )
        .and_then( | arwlctx : context::ARWLContext, headers: HeaderMap, path : warp::path::FullPath | async move
            {
                theme_file_response( arwlctx, headers, FileResponse::Common( String::from( path.as_str() ) ) ).await
            }
        );

    let r_theme =
        warp::path!( "theme" / .. )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and( warp::path::full() )
        .and_then( | arwlctx : context::ARWLContext, headers: HeaderMap, path : warp::path::FullPath | async move
            {
                theme_file_response( arwlctx, headers, FileResponse::Theme( String::from( path.as_str() ) ) ).await
            }
        );

    let r_tsound =
        warp::path!( "tsource" / .. )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and( warp::path::full() )
        .and_then( | arwlctx : context::ARWLContext, headers: HeaderMap, path : warp::path::FullPath | async move
            {
                theme_file_response( arwlctx, headers, FileResponse::Tsound( String::from( path.as_str() ) ) ).await
            }
        );

    let r_asound =
        warp::path!( "asource" / String )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and( warp::query::< AsoundParam >() )
        .and_then( | path : String, arwlctx : context::ARWLContext, headers: HeaderMap, param : AsoundParam | async move
            {
                asound_response( arwlctx, headers, path, param ).await
            }
        );

    let r_cmd  =
        warp::path!( "cmd" )
        .and( arwlctx_clone_filter() )
        .and( make_route_getpost::< CmdParam >() )
        .and_then( cmd_response );

    let r_test =
        warp::path!( "test" )
        .and( arwlctx_clone_filter() )
        .and( make_route_getpost::< HashMap< String, String > >() )
        .and_then( test_response );

    let r_status =
        warp::path!( "status" )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and_then( status_response );

    let r_spec_head =
        warp::path!( "spec_head" )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and_then( spec_head_response );

    let r_spec_data =
        warp::path!( "spec_data" )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and_then( spec_data_response );

    let r_config  =
        warp::path!( "config" )
        .and( arwlctx_clone_filter() )
        .and( make_route_getpost::< ConfigParam >() )
        .and_then( config_response );

    let r_ws =
        warp::path!( "ws" )
        .and( arwlctx_clone_filter() )
        .and( warp::ws() )
        .and( warp::addr::remote() )
        .and_then( | arwlctx : context::ARWLContext, ws: warp::ws::Ws, addr: Option< SocketAddr > |
            async move
            {
                if !arwlctx.read().await.shutdown
                {
                    Ok( ws.on_upgrade( move | ws : WebSocket | ws_response( arwlctx, ws, addr ) ) )
                }
                else
                {
                    Err( warp::reject::not_found() )
                }
            }
        );

    let r_bt_cmd  =
        warp::path!( "bt_cmd" )
        .and( arwlctx_clone_filter() )
        .and( make_route_getpost::< BtCmdParam >() )
        .and_then( bt_cmd_response );

    let r_bt_reply  =
        warp::path!( "bt_reply" )
        .and( arwlctx_clone_filter() )
        .and( make_route_getpost::< BtReplyParam >() )
        .and_then( bt_reply_response );

    let r_io_list =
        warp::path!( "io_list" )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and_then( io_list_response );

    let r_output  =
        warp::path!( "set_output" )
        .and( arwlctx_clone_filter() )
        .and( make_route_getpost::< SetOutputParam >() )
        .and_then( set_output_response );

    let r_proxy_stream  =
        warp::path!( "stream" )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and_then( | arwlctx : context::ARWLContext, headers: HeaderMap | async move
            {
                proxy_stream_response( arwlctx, headers ).await
            }
        );

    let r_album_art  =
        warp::path!( "album_art" / .. )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::header::headers_cloned() )
        .and( warp::path::full() )
        .and_then( | arwlctx : context::ARWLContext, headers: HeaderMap, path : warp::path::FullPath | async move
            {
                album_art_response( arwlctx, headers, String::from( path.as_str() ) ).await
            }
        );

    let routes =
        r_root
        .or( r_favicon )
        .or( r_theme )
        .or( r_common )
        .or( r_tsound )
        .or( r_asound )
        .or( r_cmd )
        .or( r_status )
        .or( r_spec_head )
        .or( r_spec_data )
        .or( r_config )
        .or( r_ws )
        .or( r_bt_cmd )
        .or( r_bt_reply )
        .or( r_io_list )
        .or( r_output )
        .or( r_proxy_stream )
        .or( r_album_art )
        .or( r_test )
        ;

    let with_server = warp::reply::with::header( header::SERVER, format!( "{}/{}", &product, &version ) );
    let with_log    = warp::log( "hidamari" );

    routes.with( with_server ).with( with_log ).boxed()
}

///
const PKG_NAME :    &'static str = env!( "CARGO_PKG_NAME" );
///
const PKG_VERSION : &'static str = env!( "CARGO_PKG_VERSION") ;
///
const _PKG_AUTHORS : &'static str = env!( "CARGO_PKG_AUTHORS" );

///
#[tokio::main]
async fn main() -> std::io::Result< () >
{
    std::env::set_var( "LIBASOUND_THREAD_SAFE", "0" );              // for bluealsa and ALSA thread-safe API (alsa-lib >= 1.1.2).

    if let None = std::env::var_os( "RUST_LOG" )
    {
        std::env::set_var(
            "RUST_LOG"
        ,   if cfg!(debug_assertions)
            {
                "debug,hyper=info"
            }
            else
            {
                "info"
            }
        );
    }

    pretty_env_logger::init();

    let config = context::get_config();

    if config.is_none()
    {
        return Err( std::io::Error::new( std::io::ErrorKind::Other, "stop!" ) );
    }

    let config = config.unwrap();
    let config_dyn = context::get_config_dyn( &config );

    let bind_addr   = config.bind_addr();
    let mpd_addr    = config.mpd_addr();

    let ( mpdcom_tx,        mpdcom_rx )         = sync::mpsc::channel::< mpdcom::MpdComRequest      >( 128 );
    let ( mpdfifo_tx,       mpdfifo_rx )        = sync::mpsc::channel::< mpdfifo::MpdfifoRequest    >( 128 );
    let ( btctrl_tx,        btctrl_rx )         = sync::mpsc::channel::< btctrl::BtctrlRequest      >( 128 );
    let ( bt_agent_io_tx,   bt_agent_io_rx )    = sync::mpsc::channel::< btctrl::BtctrlRepryType    >( 16 );

    let arwlctx =
        Arc::new(
            sync::RwLock::new(
                context::Context::new( config, config_dyn, mpdcom_tx, mpdfifo_tx, btctrl_tx, bt_agent_io_tx, PKG_NAME, PKG_VERSION )
            )
        );

    log::info!( "mpdcom_task start. mpd_addr {:?}", mpd_addr );

    let arwlctx_c = arwlctx.clone();
    let h_mpdcom : task::JoinHandle< _ > = task::spawn( mpdcom::mpdcom_task( arwlctx_c, mpdcom_rx ) );

    log::info!( "mpdfifo_task start. " );

    let arwlctx_c = arwlctx.clone();
    let h_mpdfifo : task::JoinHandle< _ > = task::spawn( mpdfifo::mpdfifo_task( arwlctx_c, mpdfifo_rx ) );

    let arwlctx_c = arwlctx.clone();
    let h_btctrl : task::JoinHandle< _ > = task::spawn( btctrl::btctrl_task( arwlctx_c, btctrl_rx, bt_agent_io_rx ) );

    let ( mut iolist_tx,   iolist_rx ) = event::make_channel();

    let arwlctx_c = arwlctx.clone();
    let h_iolist : task::JoinHandle< _ > = task::spawn( iolist::iolist_task( arwlctx_c, iolist_rx ) );

    log::debug!( "http server init." );

    let ( tx, rx ) = sync::oneshot::channel();

    let ( addr, server ) =
        warp::serve( make_route( arwlctx.clone() ).await )
        .bind_with_graceful_shutdown(
            bind_addr
        ,   async
            {
                let _ = rx.await.ok();
            }
        );

    log::info!( "http server start. bind_addr {:?}", addr );

    let h_server : task::JoinHandle< _ > = task::spawn( server );

    signal::ctrl_c().await?;

    arwlctx.write().await.shutdown = true;

    log::info!( "" );

    arwlctx.read().await.sdf_shutdown();

    {
        log::debug!( "ws count {}", arwlctx.read().await.ws_sessions.len() );

        let mut rxvec = Vec::< ( event::EventResultReceiver, String ) >::new();

        {
            let mut ctx = arwlctx.write().await;

            ctx.ws_sess_stop = true;

            for ( _, wss ) in ctx.ws_sessions.iter_mut()
            {
                let ( mut req, rx ) = event::new_request();

                rxvec.push( ( rx, String::from( &wss.ws_sig ) ) );
                req.req = event::EventRequestType::Shutdown;
                let _ = wss.ev_tx.send( req ).await;
            }
        }

        for x in rxvec
        {
            let _ = x.0.await;
            log::debug!( "ws shutdown. {}", x.1 );
        }

        time::delay_for( time::Duration::from_millis( 250 ) ).await;
    }

    let _ = tx.send( () );
    let _ = join!( h_server );
    log::info!( "http server shutdown." );

    arwlctx.read().await.sdf_shutdown();

    {
        let mut ctx = arwlctx.write().await;

        if ctx.bt_agent_io_rx_opend
        {
            let _ = ctx.bt_agent_io_tx.send( btctrl::BtctrlRepryType::Shutdown ).await;
        }
    }

    let ( mut req, _ ) = event::new_request();
    req.req = event::EventRequestType::Shutdown;
    let _ = iolist_tx.send( req ).await;
    let _ = join!( h_iolist );
    log::info!( "iolist_task shutdown." );

    let ( mut req, _ ) = btctrl::BtctrlRequest::new();
    req.req = btctrl::BtctrlRequestType::Shutdown;
    let _ = arwlctx.write().await.btctrl_tx.send( req ).await;
    let _ = join!( h_btctrl );
    log::info!( "btctrl_task shutdown." );

    let ( mut req, _ ) = mpdfifo::MpdfifoRequest::new();
    req.req = mpdfifo::MpdfifoRequestType::Shutdown;
    let _ = arwlctx.write().await.mpdfifo_tx.send( req ).await;
    let _ = join!( h_mpdfifo );
    log::info!( "mpdfifo_task shutdown." );

    let ( mut req, _ ) = mpdcom::MpdComRequest::new();
    req.req = mpdcom::MpdComRequestType::Shutdown;
    let _ = arwlctx.write().await.mpdcom_tx.send( req ).await;
    let _ = join!( h_mpdcom );
    log::info!( "mpdcom_task shutdown." );

    log::debug!( "ws count {}", arwlctx.read().await.ws_sessions.len() );

    let ctx = arwlctx.read().await;
    context::save_config_dyn( &ctx.config, &ctx.config_dyn );

    Ok(())
}
