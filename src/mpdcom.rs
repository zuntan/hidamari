//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::fmt;
use std::str::FromStr;
use std::net::SocketAddr;

use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::time::{ timeout, Duration, Instant };
use tokio::sync::{ oneshot, mpsc };
use tokio::prelude::*;

use serde::{ Serialize, Deserialize };

use crate::context;
use crate::event;

///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MpdComOk
{
    pub flds:       Vec< ( String, String ) >
,   pub bin:        Option< Vec< u8 > >
}

///
impl MpdComOk
{
    pub fn new() -> MpdComOk
    {
        MpdComOk { flds : Vec::new(), bin : None }
    }
}

///
#[derive(Debug, Serialize, Clone)]
pub struct MpdComOkStatus
{
    pub status:     Vec< ( String,String ) >
}

///
impl MpdComOkStatus
{
    pub fn from( f : MpdComOk ) -> MpdComOkStatus
    {
        MpdComOkStatus
        {
            status : Vec::from( f.flds )
        }
    }
}

///
#[derive(Debug, Serialize, Clone)]
pub struct MpdComErr
{
    pub err_code:   i32
,   pub cmd_index:  i32
,   pub cur_cmd:    String
,   pub msg_text:   String
}

///
impl MpdComErr
{
    pub fn new( err_code : i32 ) -> MpdComErr
    {
        MpdComErr{
            err_code
        ,   cmd_index   : 0
        ,   cur_cmd     : String::new()
        ,   msg_text    : String::new()
        }
    }
}

///
impl fmt::Display for MpdComErr
{
    fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
    {
        write!( f, "code:{} msg:{}", self.err_code, self.msg_text )
    }
}

///
pub type MpdComResult       = Result< MpdComOk,         MpdComErr >;
///
pub type MpdComStatusResult = Result< MpdComOkStatus,   MpdComErr >;

///
#[derive(Debug)]
pub enum MpdComRequestType
{
    Nop
,   Cmd( String )
,   CmdInner( String )
,   SetVol( String )
,   SetMute( String )
,   AddUrl( String, String )
,   AddAuxIn( String, String )
,   TestSound
,   Shutdown
}

///
pub struct MpdComRequest
{
    pub req  : MpdComRequestType
,   pub tx   : oneshot::Sender< MpdComResult >
}

///
impl MpdComRequest
{
    pub fn new() -> ( MpdComRequest, oneshot::Receiver< MpdComResult > )
    {
        let ( tx, rx ) = oneshot::channel::< MpdComResult >();

        (
            MpdComRequest{
                req         : MpdComRequestType::Nop
            ,   tx
            }
        ,   rx
        )
    }
}

///
pub fn quote_arg( arg: &str ) -> String
{
    let mut arg = String::from( arg.replace( '\\', r"\\" ).replace( '"', r#"\""# ) );

    if arg.contains( ' ' )
    {
        arg = String::from( "\"" ) + &arg + "\""
    }

    arg
}

///
pub fn quote_arg_f( arg: &str ) -> String
{
    String::from( "\"" ) + &String::from( arg.replace( '\\', r"\\" ).replace( '"', r#"\""# ) ) + "\""
}

struct MpdcomExec
{
    addr        : SocketAddr
,   conn        : Option< TcpStream >
}

///
impl MpdcomExec
{
    async fn exec( &mut self, cmd : String, protolog : bool )
    -> io::Result< MpdComResult >
    {
        if protolog
        {
            log::debug!( "> {}", cmd );
        }

        let conn = self.conn.as_mut().unwrap();

        conn.write( cmd.as_bytes() ).await?;
        conn.write( &[0x0a] ).await?;
        conn.flush().await?;

        let mut is_ok = false;
        let mut ret_ok = MpdComOk::new();
        let mut ret_err = MpdComErr::new( -1 );

        let mut reader = BufReader::new( conn );
        let mut buf = String::new();

        'outer: loop
        {
            buf.clear();

            if let Ok( x ) = reader.read_line( &mut buf ).await
            {
                if x == 0
                {
                    break 'outer;
                }
            }

            if protolog
            {
                log::debug!( "< {}", buf.trim_end() );
            }

            if buf == "OK\n"
            {
                is_ok = true;
                break 'outer;
            }
            else if buf.starts_with( "ACK [" )
            {
                lazy_static!
                {
                    static ref RE : regex::Regex =
                        regex::Regex::new( r"^ACK\s*\[(\d+)@(\d+)\]\s+\{([^}]*)\}\s*(.*)\n" ).unwrap();
                }

                if let Some( x ) = RE.captures( &buf )
                {
                    ret_err.err_code    = x[1].parse().unwrap();
                    ret_err.cmd_index   = x[2].parse().unwrap();
                    ret_err.cur_cmd     = String::from( &x[3] );
                    ret_err.msg_text    = String::from( &x[4] );

                    break 'outer;
                }
            }
            else
            {
                lazy_static!
                {
                    static ref RE : regex::Regex =
                        regex::Regex::new( r"^([^:]*):\s*(.*)\n" ).unwrap();
                }

                if let Some( x ) = RE.captures( &buf )
                {
                    if &x[1] == "binary"
                    {
                        let binlen : usize = x[2].parse().unwrap();
                        let mut bin = Vec::<u8>::with_capacity( binlen );

                        let mut buf = [0u8; 2048];

                        if let Ok( x ) = reader.read( &mut buf ).await
                        {
                            if x == 0
                            {
                                break 'outer;
                            }
                            else
                            {
                                bin.extend_from_slice( &buf[0..x] );
                            }
                        }

                        ret_ok.bin = Some( bin );
                    }
                    else
                    {
                        ret_ok.flds.push(
                            (
                                String::from( x[1].trim() )
                            ,   String::from( x[2].trim() )
                            )
                        );
                    }
                }
            }
        }

        if protolog && !is_ok
        {
            log::error!( "< {:?}", ret_err );
        }

        Ok( if is_ok { Ok( ret_ok ) } else { Err( ret_err ) } )
    }
}

///
pub async fn mpdcom_task(
    arwlctx : context::ARWLContext
,   mut rx  : mpsc::Receiver< MpdComRequest >
)
{
    log::debug!( "mpdcom starting." );

    let ( mut mpd_exec, mpd_protolog ) =
    {
        let ctx = arwlctx.read().await;

        (
            MpdcomExec
            {
                addr        : ctx.config.mpd_addr()
            ,   conn        : None
            }
        ,   ctx.config.mpd_protolog
        )
    };

    let mut conn_try_time : Option< Instant > = None;
    let     conn_err_retry = Duration::from_secs( 10 );

    let mut _mpd_version : Option< String > = None;

    let mut status_try_time : Option< Instant > = None;
    let status_time_out = Duration::from_millis( 250 );

    log::debug!( "mpdcom {:?} protolog {:?}", mpd_exec.addr, mpd_protolog );

    loop
    {
        if mpd_exec.conn.is_none() &&
            (   conn_try_time.is_none()
            ||  conn_try_time.unwrap().elapsed() > conn_err_retry
            )
        {
            // try connection

            conn_try_time = Some( Instant::now() );

            match TcpStream::connect( &mpd_exec.addr ).await
            {
                Ok( mut x ) =>
                {
                    let mut reader = BufReader::new( &mut x );
                    let mut buf = String::new();

                    let _ = reader.read_line( &mut buf ).await;

                    log::info!( "connected {}", &buf );

                    if !buf.starts_with("OK MPD ")
                    {
                        log::warn!( "connect shutdown" );
                        let _ = x.shutdown( std::net::Shutdown::Both );
                    }
                    else
                    {
                        mpd_exec.conn = Some( x );
                        _mpd_version = Some( String::from( buf[7..].trim() ) )
                    }
                }
                Err( x ) =>
                {
                    log::warn!( "connect error [{:?}]", x );
                }
            }
        }

        if mpd_exec.conn.is_some() &&
            (   status_try_time.is_none()
            ||  status_try_time.unwrap().elapsed() > status_time_out
            )
        {
            let mut status_ok = false;

            match mpd_exec.exec( String::from( "status" ), false ).await
            {
                Ok(mut x) =>
                {
                    if let Ok( x2 ) = x.as_mut()
                    {
                        {
                            let ctx = arwlctx.read().await;

                            x2.flds.push( ( String::from( "_x_time" ),              chrono::Local::now().to_rfc3339() ) );
                            x2.flds.push( ( String::from( "_x_product" ),           String::from( &ctx.product ) ) );
                            x2.flds.push( ( String::from( "_x_version" ),           String::from( &ctx.version ) ) );
                            x2.flds.push( ( String::from( "_x_ws_status_intv" ),    format!( "{:?}", &ctx.ws_status_intv ) ) );
                            x2.flds.push( ( String::from( "_x_ws_data_intv" ),      format!( "{:?}", &ctx.ws_data_intv ) ) );
                            x2.flds.push( ( String::from( "_x_spec_enable" ),       String::from( if ctx.spec_enable { "1" } else { "0" } ) ) );
                        }

                        let p = x2.flds.iter().position( |x| x.0 == "volume" );

                        if let Some( p ) = p
                        {
                            let vol = x2.flds.remove( p );

                            let volval = if let Ok( volval ) = u8::from_str( &vol.1 ) { volval } else { 0 };

                            let mut ctx = arwlctx.write().await;

                            if !ctx.mpd_mute || volval > 0
                            {
                                ctx.mpd_volume = volval;
                                ctx.mpd_mute = false;
                            }

                            x2.flds.push( ( String::from( "volume"      ), ctx.mpd_volume.to_string() ) );
                            x2.flds.push( ( String::from( "mute"        ), String::from( if ctx.mpd_mute { "1" } else { "0" } ) ) );
                            x2.flds.push( ( String::from( "_x_volume"   ), vol.1 ) );
                        }
                    }

                    let mut songids = Vec::<String>::new();

                    if let Ok( x ) = x.as_ref()
                    {
                        for( k, v ) in x.flds.iter()
                        {
                            if k == "songid" || k == "nextsongid"
                            {
                                songids.push( String::from( v ) );
                            }
                        }
                    }

                    if let Ok( x2 ) = x.as_mut()
                    {
                        for si in songids
                        {
                            match mpd_exec.exec( format!( "playlistid {}", si ), false ).await
                            {
                                Ok( x1 ) =>
                                {
                                    if let Ok( x1 ) = x1
                                    {
                                        x2.flds.extend_from_slice( &x1.flds );
                                    }
                                }
                            ,   Err(_) => {}
                            }
                        }
                    }

                    let mut ctx = arwlctx.write().await;

                    ctx.mpd_status_json =
                        match x
                        {
                            Ok( x2 ) =>
                            {
                                match serde_json::to_string( &MpdComStatusResult::Ok( MpdComOkStatus::from( x2 ) ) )
                                {
                                    Ok( x ) => { x }
                                ,   _       => { String::new() }
                                }
                            }
                            Err( ref x2 ) =>
                            {
                                match serde_json::to_string( x2 )
                                {
                                    Ok( x ) => { x }
                                    _       => { String::new() }
                                }
                            }
                        };

                    status_try_time = Some( Instant::now() );
                    status_ok = true;
                }
            ,   Err(x) =>
                {
                    log::warn!( "connection error [{:?}]", x );
                    mpd_exec.conn.as_mut().unwrap().shutdown();
                    mpd_exec.conn = None;
                    conn_try_time = Some( Instant::now() );
                }
            }

            if !status_ok
            {
                let mut ctx = arwlctx.write().await;

                ctx.mpd_status_json =
                    match serde_json::to_string( &MpdComResult::Err( MpdComErr::new( -1 ) )  )
                    {
                        Ok( x ) => { x }
                        _       => { String::new() }
                    }
                    ;
            }
        }

        match timeout( event::EVENT_WAIT_TIMEOUT, rx.recv() ).await
        {
            Ok(recv) =>
            {
                let recv = recv.unwrap();

                if let MpdComRequestType::CmdInner( ref _cmd ) = recv.req
                {
                }
                else
                {
                    log::debug!( "recv [{:?}]", recv.req );
                }

                match recv.req
                {
                    MpdComRequestType::Shutdown =>
                    {
                        if mpd_exec.conn.is_some()
                        {
                            log::info!( "connection close" );
                            mpd_exec.conn.as_mut().unwrap().shutdown();
                        }

                        recv.tx.send( Ok( MpdComOk::new() ) ).ok();
                        break;
                    }

                ,   MpdComRequestType::SetVol( volume ) =>
                    {
                        if let Ok( vol ) = u8::from_str( &volume )
                        {
                            if vol <= 100
                            {
                                let mut done = false;

                                {
                                    let mut ctx = arwlctx.write().await;

                                    if ctx.mpd_mute
                                    {
                                        ctx.mpd_volume = vol;
                                        done = true;
                                    }
                                }

                                if !done
                                {
                                    let cmd = String::from( "setvol " ) + &vol.to_string();

                                    match mpd_exec.exec( cmd, mpd_protolog ).await
                                    {
                                        Ok(x) =>
                                        {
                                            recv.tx.send( x ).ok();
                                        }
                                    ,   Err(x) =>
                                        {
                                            log::warn!( "connection error [{:?}]", x );
                                            mpd_exec.conn.as_mut().unwrap().shutdown();
                                            mpd_exec.conn = None;
                                            conn_try_time = Some( Instant::now() );

                                            recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
                                        }
                                    }
                                }
                                else
                                {
                                    recv.tx.send( Ok( MpdComOk::new() ) ).ok();
                                }
                            }
                            else
                            {
                                recv.tx.send( Err( MpdComErr::new( -3 ) ) ).ok();
                            }
                        }
                        else
                        {
                            recv.tx.send( Err( MpdComErr::new( -3 ) ) ).ok();
                        }
                    }

                ,   MpdComRequestType::SetMute( mute ) =>
                    {
                        let mute = match mute.to_lowercase().as_str()
                            {
                                "1" | "true" | "on" => true
                            ,   _                   => false
                            };

                        let cmd = if mute
                            {
                                String::from( "setvol 0" )
                            }
                            else
                            {
                                let ctx = arwlctx.read().await;
                                String::from( "setvol " ) + &ctx.mpd_volume.to_string()
                            };

                        match mpd_exec.exec( cmd, mpd_protolog ).await
                        {
                            Ok(x) =>
                            {
                                let mut ctx = arwlctx.write().await;
                                ctx.mpd_mute = mute;

                                recv.tx.send( x ).ok();
                            }
                        ,   Err(x) =>
                            {
                                log::warn!( "connection error [{:?}]", x );
                                mpd_exec.conn.as_mut().unwrap().shutdown();
                                mpd_exec.conn = None;
                                conn_try_time = Some( Instant::now() );

                                recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
                            }
                        }
                    }

                ,   MpdComRequestType::AddUrl( url, append ) =>
                    {
                        let append = match append.to_lowercase().as_str()
                            {
                                "1" | "true" | "on" => true
                            ,   _                   => false
                            };

                        match context::check_url( &url )
                        {
                            Ok(_) =>
                            {
                                let cmd = String::from( "addid " ) + &quote_arg( &url );

                                log::debug!( "addid {}", &url );

                                match mpd_exec.exec( cmd, mpd_protolog ).await
                                {
                                    Ok(x) =>
                                    {
                                        if append
                                        {
                                            arwlctx.write().await.append_url( &url );
                                        }

                                        recv.tx.send( x ).ok();
                                    }
                                ,   Err(x) =>
                                    {
                                        log::warn!( "connection error [{:?}]", x );
                                        mpd_exec.conn.as_mut().unwrap().shutdown();
                                        mpd_exec.conn = None;
                                        conn_try_time = Some( Instant::now() );

                                        recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
                                    }
                                }
                            }
                        ,   Err(x) =>
                            {
                                let mut err = MpdComErr::new( -4 );

                                err.msg_text = String::from( format!( "{:?}", x ) );

                                recv.tx.send( Err( err ) ).ok();
                            }
                        }
                    }

                ,   MpdComRequestType::AddAuxIn( url, name ) =>
                    {
                        let mut ret_ok = MpdComOk::new();
                        let mut ret_err : Option< MpdComErr > = None;

                        let url = arwlctx.read().await.update_hidamari_url( &url );

                        let cmd = String::from( "addid " ) + &quote_arg( &url );

                        log::debug!( "addauxin {}", &cmd );

                        match mpd_exec.exec( cmd, mpd_protolog ).await
                        {
                            Ok( x ) =>
                            {
                                match x
                                {
                                    Ok( mut x ) =>
                                    {
                                        ret_ok.flds.push( ( String::from( "file" ), String::from( &url ) ) );
                                        ret_ok.flds.append( &mut x.flds );

                                        if name != ""
                                        {
                                            if let Some( x ) = ret_ok.flds.iter().find( | x | x.0 == "Id" )
                                            {
                                                let id = &x.1;

                                                let cmd = String::from( "addtagid " ) + &quote_arg( id ) + r#" "Title" "# + &quote_arg_f( &name );

                                                log::debug!( "addauxin [{}]", &cmd );

                                                match mpd_exec.exec( cmd, mpd_protolog ).await
                                                {
                                                    Ok( x ) =>
                                                    {
                                                        match x
                                                        {
                                                            Ok( _ ) =>
                                                            {
                                                                ret_ok.flds.push( ( String::from( "Title" ), String::from( &name ) ) );
                                                            }
                                                        ,   Err( x ) =>
                                                            {
                                                                log::warn!( "error [{:?}]", x );
                                                                ret_err = Some( x );
                                                            }
                                                        }
                                                    }
                                                ,   Err(x) =>
                                                    {
                                                        log::warn!( "connection error [{:?}]", x );
                                                        mpd_exec.conn.as_mut().unwrap().shutdown();
                                                        mpd_exec.conn = None;
                                                        conn_try_time = Some( Instant::now() );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                ,   Err( x ) =>
                                    {
                                        log::warn!( "error [{:?}]", x );
                                        ret_err = Some( x );
                                    }
                                }
                            }
                        ,   Err(x) =>
                            {
                                log::warn!( "connection error [{:?}]", x );
                                mpd_exec.conn.as_mut().unwrap().shutdown();
                                mpd_exec.conn = None;
                                conn_try_time = Some( Instant::now() );
                            }
                        }

                        if let Some( x ) = ret_err
                        {
                            recv.tx.send( Err( x ) ).ok();
                        }
                        else
                        {
                            recv.tx.send( Ok( ret_ok ) ).ok();
                        }
                    }

                ,   MpdComRequestType::TestSound =>
                    {
                        let testsound_urllist =
                        {
                            arwlctx.read().await.testsound_urllist()
                        };

                        let mut ret_ok = MpdComOk::new();
                        let mut ret_err : Option< MpdComErr > = None;

                        for ( url, title ) in testsound_urllist
                        {
                            let cmd = String::from( "addid " ) + &quote_arg( &url );

                            log::debug!( "testsound {} {}", &url, title );

                            match mpd_exec.exec( cmd, mpd_protolog ).await
                            {
                                Ok( x ) =>
                                {
                                    match x
                                    {
                                        Ok( mut x ) =>
                                        {
                                            let id =
                                                if let Some( x ) = x.flds.iter().find( | x | x.0 == "Id" )
                                                {
                                                    Some( String::from( &x.1 ) )
                                                }
                                                else
                                                {
                                                    None
                                                };

                                            ret_ok.flds.push( ( String::from( "file" ), String::from( &url ) ) );
                                            ret_ok.flds.append( &mut x.flds );

                                            if title != ""
                                            {
                                                if let Some( id ) = id
                                                {
                                                    let cmd = String::from( "addtagid " ) + &quote_arg( &id ) + r#" "Title" "# + &quote_arg_f( &title );

                                                    log::debug!( "testsound [{}]", &cmd );

                                                    match mpd_exec.exec( cmd, mpd_protolog ).await
                                                    {
                                                        Ok( x ) =>
                                                        {
                                                            match x
                                                            {
                                                                Ok( _ ) =>
                                                                {
                                                                    ret_ok.flds.push( ( String::from( "Title" ), String::from( &title ) ) );
                                                                }
                                                            ,   Err( x ) =>
                                                                {
                                                                    log::warn!( "error [{:?}]", x );
                                                                    ret_err = Some( x );
                                                                }
                                                            }
                                                        }
                                                    ,   Err(x) =>
                                                        {
                                                            log::warn!( "connection error [{:?}]", x );
                                                            mpd_exec.conn.as_mut().unwrap().shutdown();
                                                            mpd_exec.conn = None;
                                                            conn_try_time = Some( Instant::now() );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    ,   Err( x ) =>
                                        {
                                            log::warn!( "error [{:?}]", x );
                                            ret_err = Some( x );
                                            break;
                                        }
                                    }
                                }
                            ,   Err(x) =>
                                {
                                    log::warn!( "connection error [{:?}]", x );
                                    mpd_exec.conn.as_mut().unwrap().shutdown();
                                    mpd_exec.conn = None;
                                    conn_try_time = Some( Instant::now() );
                                }
                            }
                        }

                        if let Some( x ) = ret_err
                        {
                            recv.tx.send( Err( x ) ).ok();
                        }
                        else
                        {
                            recv.tx.send( Ok( ret_ok ) ).ok();
                        }
                    }

                ,   MpdComRequestType::Cmd( cmd ) =>
                    {
                        if cmd != "close" && mpd_exec.conn.is_some()
                        {
                            match mpd_exec.exec( cmd, mpd_protolog ).await
                            {
                                Ok(x) =>
                                {
                                    recv.tx.send( x ).ok();
                                }
                            ,   Err(x) =>
                                {
                                    log::warn!( "connection error [{:?}]", x );
                                    mpd_exec.conn.as_mut().unwrap().shutdown();
                                    mpd_exec.conn = None;
                                    conn_try_time = Some( Instant::now() );

                                    recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
                                }
                            }
                        }
                        else
                        {
                            recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
                        }
                    }
                ,   MpdComRequestType::CmdInner( cmd ) =>
                    {
                        if cmd != "close" && mpd_exec.conn.is_some()
                        {
                            match mpd_exec.exec( cmd, false ).await
                            {
                                Ok(x) =>
                                {
                                    recv.tx.send( x ).ok();
                                }
                            ,   Err(x) =>
                                {
                                    log::warn!( "connection error [{:?}]", x );
                                    mpd_exec.conn.as_mut().unwrap().shutdown();
                                    mpd_exec.conn = None;
                                    conn_try_time = Some( Instant::now() );

                                    recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
                                }
                            }
                        }
                        else
                        {
                            recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
                        }
                    }

                ,   _ =>
                    {
                        recv.tx.send( Err( MpdComErr::new( -3 ) ) ).ok();
                    }
                }
            }
        ,   Err(_) => {}
        }
    }

    log::debug!( "mpdcom stop." );
}

