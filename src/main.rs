//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///

extern crate pretty_env_logger;
extern crate tokio;
extern crate chrono;
extern crate headers;

#[macro_use]
extern crate lazy_static;

use std::sync::Arc;
use std::collections::HashMap;
use std::path::{ Path, PathBuf };
use std::result::Result;

use tokio::signal;
use tokio::sync;
use tokio::time;
use tokio::task;
use tokio::join;
use tokio::fs::File;
use tokio_util::codec::{ BytesCodec, FramedRead };

use futures::{ StreamExt, SinkExt };

use warp::{ Filter, filters, reply::Reply, reply::Response, reject::Rejection, hyper::Body };

use headers::HeaderMapExt;

use warp::http::header::{ HeaderValue, CONTENT_TYPE };
use warp::http::StatusCode;
use warp::ws::{ Message, WebSocket };

use serde::{ Serialize, Deserialize, de::DeserializeOwned };

mod config;
mod mpdcom;
mod mpdfifo;
mod event;

type WsSessions = HashMap< u64, WsSession >;

struct WsSession
{
    ws_sig  : String
,   ev_tx   : event::EventSender
}

pub struct Context
{
    config          : config::Config
,   config_dyn      : config::ConfigDyn

,   mpdcom_tx       : sync::mpsc::Sender< mpdcom::MpdComRequest >
,   mpd_status_json : String

,   mpd_volume      : u8
,   mpd_mute        : bool

,   spec_data_json  : String
,   spec_head_json  : String

,   ws_sess_no      : u64
,   ws_sessions     : WsSessions

,   ws_status_intv  : time::Duration
,   ws_data_intv    : time::Duration
,   ws_send_intv    : time::Duration

,   version         : String
}

impl Context
{
    fn new(
        config      : config::Config
    ,   config_dyn  : config::ConfigDyn
    ,   mpdcom_tx   : sync::mpsc::Sender< mpdcom::MpdComRequest >
    ,   version     : &str
    ) -> Context
    {
        Context
        {
            config
        ,   config_dyn
        ,   mpdcom_tx
        ,   mpd_status_json : String::new()
        ,   mpd_volume      : 0
        ,   mpd_mute        : false
        ,   spec_data_json  : String::new()
        ,   spec_head_json  : String::new()
        ,   ws_sess_no      : 0
        ,   ws_sessions     : WsSessions::new()
        ,   ws_status_intv  : time::Duration::from_millis( 200 )
        ,   ws_data_intv    : time::Duration::from_millis( 200 )
        ,   ws_send_intv    : time::Duration::from_secs( 3 )
        ,   version         : String::from( version )
        }
    }

    fn get_theme_path( &self ) -> PathBuf
    {
        let mut path = PathBuf::new();

        if self.config.theme_dir != ""
        {
            path.push( &self.config.theme_dir );
        }
        else
        {
            path.push( config::THEME_DIR );
        }

        if self.config_dyn.theme != ""
        {
            path.push( &self.config_dyn.theme );
        }

        path
    }

    fn get_common_path( &self ) -> PathBuf
    {
        let mut path = PathBuf::new();

        if self.config.theme_dir != ""
        {
            path.push( &self.config.theme_dir );
        }
        else
        {
            path.push( config::THEME_DIR );
        }

        path.push( config::THEME_COMMON_DIR );

        path
    }
}

///
type ARWLContext = Arc< sync::RwLock< Context > >;

///
type StrResult = Result< String, Rejection >;

///
type RespResult = Result< Response, Rejection >;

fn json_response< T: ?Sized + Serialize >( t : &T ) -> Response
{
    let mut r = Response::new(
        match serde_json::to_string( t )
        {
            Ok( x ) => { x }
        ,   _       => { String::new() }
        }.into()
    );
    r.headers_mut().insert( CONTENT_TYPE, HeaderValue::from_static( "application/json" ) );
    r
}

fn internal_server_error( t : &str ) -> Response
{
    let mut r = Response::new( String::from( t ).into() );
    *r.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    r
}

///
async fn make_file_response( path: &Path ) -> RespResult
{
    match File::open( path ).await
    {
        Ok( file ) =>
        {
            let metadata = file.metadata().await;

            let stream = FramedRead::new( file, BytesCodec::new() );
            let body = Body::wrap_stream( stream );
            let mut resp = Response::new( body );

            let mime = mime_guess::from_path( path ).first_or_octet_stream();

            if let Ok( metadata ) = metadata
            {
                resp.headers_mut().typed_insert( headers::ContentLength( metadata.len() ) );
            }

            resp.headers_mut().typed_insert( headers::ContentType::from( mime ) );

            return Ok( resp );
        }
    ,   Err( x ) =>
        {
            log::warn!( "{:?}", x );
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

async fn theme_file_response( arwlctx : ARWLContext, path : &str, is_common : bool, do_unshift : bool ) -> RespResult
{
    let path =
    {
        if do_unshift
        {
            path.split( '/' )
                .skip( 1 )
                .map( |x| x.to_string() )
                .collect::< Vec< String > >()
                .join( "/" )
        }
        else
        {
            String::from( path )
        }
    };

    match check_path( &path )
    {
        Err( x ) => { RespResult::Err( x ) }
    ,   Ok( path ) =>
        {
            let mut path_base =
            {
                if is_common
                {
                    arwlctx.read().await.get_common_path()
                }
                else
                {
                    arwlctx.read().await.get_theme_path()
                }
            };

            path_base.push( &path );

            make_file_response( &path_base ).await
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
            ,   "" =>
                {
                    mpdcom::MpdComRequestType::Nop
                }
            ,   _ =>
                {
                    if self.arg1.is_some()
                    {
                        cmd += " ";
                        cmd += &mpdcom::quote_arg( self.arg1.as_ref().unwrap().as_str() );
                    }

                    if self.arg2.is_some()
                    {
                        cmd += " ";
                        cmd += &mpdcom::quote_arg( self.arg2.as_ref().unwrap().as_str() );
                    }

                    if self.arg3.is_some()
                    {
                        cmd += " ";
                        cmd += &mpdcom::quote_arg( self.arg3.as_ref().unwrap().as_str() );
                    }

                    mpdcom::MpdComRequestType::Cmd( cmd )
                }
            };

        let ( mut req, rx ) = mpdcom::MpdComRequest::new();

        req.req = reqval;

        ( req, rx )
    }
}

async fn cmd_response( arwlctx : ARWLContext, param : CmdParam ) -> RespResult
{
    log::debug!( "{:?}", &param );

    let ( req, rx ) = param.to_request();

    let _ = arwlctx.write().await.mpdcom_tx.send( req ).await;

    Ok(
        match rx.await
        {
            Ok(x)  => json_response( &x )
        ,   Err(x) => internal_server_error( &format!( "{:?}", x ) )
        }
    )
}

async fn status_response( arwlctx : ARWLContext ) -> RespResult
{
    Ok( json_response( &arwlctx.read().await.mpd_status_json ) )
}

async fn spec_head_response( arwlctx : ARWLContext ) -> RespResult
{
    Ok( json_response( &arwlctx.read().await.spec_head_json ) )
}

async fn spec_data_response( arwlctx : ARWLContext ) -> RespResult
{
    Ok( json_response( &arwlctx.read().await.spec_data_json ) )
}

///
#[derive(Debug, Deserialize, Clone)]
struct ConfigParam
{
    update : Option<String>
}

///
async fn config_response( arwlctx : ARWLContext, param : ConfigParam ) -> RespResult
{
    if param.update.is_some()
    {
        let mut ctx = arwlctx.write().await;

        let newval = String::from( param.update.as_ref().unwrap().trim_end() );

        if newval != ""
        {
            ctx.config_dyn.update( &newval );
        }
    }

    let ctx = arwlctx.read().await;

    Ok( json_response( &config::make_config_dyn_output( &ctx.config, &ctx.config_dyn ) ) )
}

async fn ws_response( arwlctx : ARWLContext, ws : WebSocket )
{
    let (
        ws_no
    ,   ws_sig
    ,   mut ev_rx
    ,   ws_status_intv
    ,   ws_data_intv
    ,   ws_send_intv
    ,   mut last_mpd_status_json
    ,       last_spec_head_json
    ,   mut last_spec_data_json
    ) =
    {
        let mut ctx = arwlctx.write().await;

        ctx.ws_sess_no += 1;

        let ws_no = ctx.ws_sess_no;
        let ws_sig = format!( "ws:{}:{:?}", ws_no, &ws );

        let ( ev_tx, ev_rx ) = event::make_channel();

        ctx.ws_sessions.insert( ws_no, WsSession{ ws_sig : String::from( &ws_sig ), ev_tx } );

        (
            ws_no
        ,   ws_sig
        ,   ev_rx
        ,   ctx.ws_status_intv
        ,   ctx.ws_data_intv
        ,   ctx.ws_send_intv
        ,   String::from( &ctx.mpd_status_json )
        ,   String::from( &ctx.spec_head_json )
        ,   String::from( &ctx.spec_data_json )
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

    let ( mut ws_tx, mut ws_rx ) = ws.split();

    if let Err(x) = ws_tx.send( Message::text( &last_mpd_status_json ) ).await
    {
        log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
        cleanup!();
        return;
    }

    if let Err(x) = ws_tx.send( Message::text( &last_spec_head_json ) ).await
    {
        log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
        cleanup!();
        return;
    }

    if let Err(x) = ws_tx.send( Message::text( &last_spec_data_json ) ).await
    {
        log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
        cleanup!();
        return;
    }

    let mut last_check_status = time::Instant::now();
    let mut last_send_status  = time::Instant::now();

    let mut last_check_data   = time::Instant::now();
    let mut last_send_data    = time::Instant::now();

    loop
    {
        if event::event_shutdown( &mut ev_rx ).await
        {
            break;
        }

        if let Ok( r ) =  time::timeout( event::EVENT_WAIT_TIMEOUT, ws_rx.next() ).await
        {
            if let Some( recv ) = r
            {
                match recv
                {
                    Err( e ) =>
                    {
                        log::warn!( "web socket error. {:?} {:?}", &e, &ws_sig );
                    }
                ,   Ok( x ) =>
                    {
                        log::debug!( "web socket recv. {:?} {:?}", &x, &ws_sig );
                    }
                }
            }
        }

        if last_check_status.elapsed() > ws_status_intv
        {
            last_check_status = time::Instant::now();

            if
            {
                let ctx = arwlctx.read().await;

                if ctx.mpd_status_json != last_mpd_status_json
                {
                    last_mpd_status_json = String::from( &ctx.mpd_status_json );
                    true
                }
                else
                {
                    false
                }
            } || last_send_status.elapsed() > ws_send_intv
            {
                last_send_status = time::Instant::now();

                if let Err(x) = ws_tx.send( Message::text( &last_mpd_status_json ) ).await
                {
                    log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
                    break;
                }
            }
        }

        if last_check_data.elapsed() > ws_data_intv
        {
            last_check_data = time::Instant::now();

            if
            {
                let ctx = arwlctx.read().await;

                if ctx.spec_data_json != last_spec_data_json
                {
                    last_spec_data_json = String::from( &ctx.spec_data_json );
                    true
                }
                else
                {
                    false
                }
            } || last_send_data.elapsed() > ws_send_intv
            {
                last_send_data = time::Instant::now();

                if let Err(x) = ws_tx.send( Message::text( &last_spec_data_json ) ).await
                {
                    log::debug!( "web socket error. {:?} {:?}", &x, &ws_sig );
                    break;
                }
            }
        }
    }

    cleanup!();
}

async fn test_response( arwlctx : ARWLContext, param : HashMap< String, String > ) -> StrResult
{
    StrResult::Ok( String::new() )
}

///
fn make_route( arwlctx : ARWLContext )
    -> filters::BoxedFilter< ( impl Reply, ) >
{
    let arwlctx_clone_filter = move ||
        {
            let x_arwlctx = arwlctx.clone();
            warp::any().map( move || x_arwlctx.clone() )
        };

    let r_root =
        warp::path::end()
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and_then( | arwlctx : ARWLContext | async move
            {
                theme_file_response( arwlctx, config::THEME_MAIN, false, false ).await
            }
        );

    let r_favicon =
        warp::path!( "favicon.ico" )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and_then( | arwlctx : ARWLContext | async move
            {
                theme_file_response( arwlctx, "favicon.ico", false, false ).await
            }
        );

    let r_common =
        warp::path!( "common" / .. )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::path::full() )
        .and_then( | arwlctx : ARWLContext, path : warp::path::FullPath | async move
            {
                theme_file_response( arwlctx, path.as_str(), true, true ).await
            }
        );

    let r_theme =
        warp::path!( "theme" / .. )
        .and( arwlctx_clone_filter() )
        .and( warp::get() )
        .and( warp::path::full() )
        .and_then( | arwlctx : ARWLContext, path : warp::path::FullPath | async move
            {
                theme_file_response( arwlctx, path.as_str(), false, true ).await
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
        .map( | arwlctx : ARWLContext, ws: warp::ws::Ws |
            {
                ws.on_upgrade( move | ws : WebSocket | ws_response( arwlctx, ws ) )
            }
        );

    let routes =
        r_root
        .or( r_favicon )
        .or( r_theme )
        .or( r_common )
        .or( r_cmd )
        .or( r_status )
        .or( r_spec_head )
        .or( r_spec_data )
        .or( r_config )
        .or( r_ws )
        .or( r_test )
        ;

    let with_server = warp::reply::with::header( "server", "hidamari" );
    let with_log    = warp::log( "hidamari" );

    routes.with( with_server ).with( with_log ).boxed()
}

const PKG_NAME :    &'static str = env!( "CARGO_PKG_NAME" );
const PKG_VERSION : &'static str = env!( "CARGO_PKG_VERSION") ;
const PKG_AUTHORS : &'static str = env!( "CARGO_PKG_AUTHORS" );

///
#[tokio::main]
async fn main() -> std::io::Result< () >
{
    std::env::set_var( "RUST_LOG", "debug,hyper=info" );

    pretty_env_logger::init();

    let config = config::get_config();

    if config.is_none()
    {
        return Err( std::io::Error::new( std::io::ErrorKind::Other, "stop!" ) );
    }

    let config = config.unwrap();
    let config_dyn = config::get_config_dyn( &config );

    let bind_addr   = config.bind_addr();
    let mpd_addr    = config.mpd_addr();

    let ( mpdcom_tx,    mpdcom_rx )     = sync::mpsc::channel::< mpdcom::MpdComRequest >( 128 );

    let arwlctx =
        Arc::new(
            sync::RwLock::new(
                Context::new( config, config_dyn, mpdcom_tx, PKG_VERSION )
            )
        );

    log::info!( "mpdcom_task start. mpd_addr {:?}", mpd_addr );

    let arwlctx_c = arwlctx.clone();
    let h_mpdcom : task::JoinHandle< _ > = task::spawn( mpdcom::mpdcom_task( arwlctx_c, mpdcom_rx ) );

    log::info!( "mpdfifo_task start. " );

    let ( mut mpdfifo_tx,   mpdfifo_rx ) = event::make_channel();

    let arwlctx_c = arwlctx.clone();
    let h_mpdfifo : task::JoinHandle< _ > = task::spawn( mpdfifo::mpdfifo_task( arwlctx_c, mpdfifo_rx ) );

    log::debug!( "http server init." );

    let ( tx, rx ) = sync::oneshot::channel();

    let ( addr, server ) =
        warp::serve( make_route( arwlctx.clone() ) )
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

    log::info!( "" );

    let _ = tx.send( () );
    let _ = join!( h_server );
    log::info!( "http server shutdown." );

    {
        log::debug!( "ws count {}", arwlctx.read().await.ws_sessions.len() );

        let mut rxvec = Vec::< ( event::EventResultReceiver, String ) >::new();

        {
            let mut ctx = arwlctx.write().await;

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

        log::debug!( "ws count {}", arwlctx.read().await.ws_sessions.len() );
    }

    let ( mut req, _ ) = event::new_request();
    req.req = event::EventRequestType::Shutdown;
    let _ = mpdfifo_tx.send( req ).await;
    let _ = join!( h_mpdfifo );
    log::info!( "mpdfifo_task shutdown." );

    let ( mut req, _ ) = mpdcom::MpdComRequest::new();
    req.req = mpdcom::MpdComRequestType::Shutdown;
    let _ = arwlctx.write().await.mpdcom_tx.send( req ).await;
    let _ = join!( h_mpdcom );
    log::info!( "mpdcom_task shutdown." );

    let ctx = arwlctx.read().await;
    config::save_config_dyn( &ctx.config, &ctx.config_dyn );

    Ok(())
}
