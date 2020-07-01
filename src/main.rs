//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

extern crate actix_web;
extern crate actix_utils;
extern crate actix_rt;

extern crate tokio;

extern crate chrono;

#[macro_use]
extern crate lazy_static;

use std::io;
use std::io::prelude::*;
use std::path;
use std::fs;
use std::net::ToSocketAddrs;
use std::sync::{ Mutex };
use std::collections::hash_map::{ HashMap };

// use actix_web::http::{ header, Method, StatusCode };
use actix_web::{ error, /* guard, */ middleware, web };
use actix_web::{ App, HttpRequest, HttpResponse, HttpServer, Result, Error };
use actix_files as afs;
use actix_web_actors::ws;

use tokio::sync::{ oneshot, mpsc };

use serde::{ /* Serialize, */ Deserialize };

mod mpdcom;
mod wssession;
mod mpdfifo;

///
#[derive(Debug, Deserialize, Clone)]
struct Config
{
    bind_addr   : String
,   mpd_addr    : String
,   mpd_protolog: bool
,   mpd_fifo    : String
,   log_level   : String
,   theme_dir   : String
,   theme       : String
}

///
pub struct Context
{
    config              : Config
,   mpdcom_tx           : mpsc::Sender< mpdcom::MpdComRequest >
,   mpd_status          : mpdcom::MpdComResult
,   status_ws_sessions  : wssession::WsSessions

,   mpdfifo_tx          : mpsc::Sender< mpdfifo::MpdFifoRequest >
,   bar_dat_json        : String
,   bar_cap_json        : String
}

///
impl Context
{
    pub fn get_theme_path( &self ) -> path::PathBuf
    {
        let mut path = path::PathBuf::new();

        if self.config.theme_dir != ""
        {
            path.push( &self.config.theme_dir );
        }
        else
        {
            path.push( "_theme" );
        }

        if self.config.theme != ""
        {
            path.push( &self.config.theme );
        }

        path
    }
}

///
fn get_config() -> Option< Config >
{
    let mut targets = vec![
        String::from( "hidamari.conf" )
    ,   String::from( "/etc/hidamari.conf" )
    ];

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2
    {
        targets.insert( 0, args[1].clone() );
    }

    let mut conts : Option< String > = None;

    for t in targets
    {
        log::debug!( "config try loading [{:?}]", &t );

        if let Ok( mut f ) = fs::File::open( &t ).map( |f| io::BufReader::new( f ) )
        {
            log::info!( "config loading [{}]", &t );

            let mut tmp_conts = String::new();

            if let Ok( _ ) = f.read_to_string( &mut tmp_conts )
            {
                conts = Some( tmp_conts );
            }

            break;
        }
    }

    if conts.is_some()
    {
        if let Ok( x ) = toml::de::from_str::<Config>( &conts.unwrap() )
        {
            if x.bind_addr.to_socket_addrs().is_err()
            {
                log::error!( "invalid value `bind_addr`" );
            }
            else if x.mpd_addr.to_socket_addrs().is_err()
            {
                log::error!( "invalid value `mpd_addr`" );
            }
            else
            {
                return Some( x );
            }
        }
    }

    log::error!( "config load error." );
    None
}

///
fn theme_content_impl( ctx : web::Data< Mutex< Context > >, p : &path::Path ) -> Result< afs::NamedFile >
{
    let mut path = ctx.lock().unwrap().get_theme_path();

    path.push( p );

    log::debug!("{:?}", &path );

    if path.is_file()
    {
        Ok( afs::NamedFile::open( path )? )
    }
    else
    {
        return Err( error::ErrorNotFound( "" ) );
    }
}

///
async fn status_ws( ctx : web::Data< Mutex< Context > >, r: HttpRequest, stream: web::Payload ) -> Result< HttpResponse, Error >
{
    ws::start( wssession::ArcWsSession::new( &ctx, wssession::WsSwssionType::Status ), &r, stream )
}

///
async fn status( ctx : web::Data< Mutex< Context > > ) -> HttpResponse
{
    let ctx = ctx.lock().unwrap();

    HttpResponse::Ok().json( &ctx.mpd_status  )
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
    fn to_request( &self ) -> ( mpdcom::MpdComRequest, oneshot::Receiver< mpdcom::MpdComResult > )
    {
        let mut cmd = String::new();

        cmd += &self.cmd;

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

        let ( mut req, rx ) = mpdcom::MpdComRequest::new();

        req.req = mpdcom::MpdComRequestType::Cmd( cmd );

        ( req, rx )
    }
}

///
async fn cmd_impl( ctx : web::Data< Mutex< Context > >, param : &CmdParam ) -> HttpResponse
{
    log::debug!("{:?}", &param );

    let ( req, rx ) = param.to_request();

    &ctx.lock().unwrap().mpdcom_tx.send( req ).await;

    match rx.await
    {
        Ok(x) =>    { HttpResponse::Ok().json( x ) }
    ,   Err(x) =>   { HttpResponse::InternalServerError().body( format!( "{:?}", x ) ) }
    }
}

///
async fn cmd_get( ctx : web::Data< Mutex< Context > >, param: web::Query<CmdParam> ) -> HttpResponse
{
    log::debug!("command_get" );
    cmd_impl( ctx, &*param ).await
}

///
async fn cmd_post( ctx : web::Data< Mutex< Context > >, param: web::Form<CmdParam> ) -> HttpResponse
{
    log::debug!("command_post" );
    cmd_impl( ctx, &*param ).await
}


///
async fn favicon( ctx : web::Data< Mutex< Context > > ) -> Result< afs::NamedFile >
{
    theme_content_impl( ctx, path::Path::new( "favicon.ico" ) )
}

///
async fn theme( ctx : web::Data< Mutex< Context > >, req : HttpRequest ) -> Result< afs::NamedFile >
{
    log::debug!("{}", req.path() );

    let mut p : Vec<String> = Vec::new();

    for x in req.path().split( '/' ).skip(2)
    {
        match x
        {
            "" | "."    => {}
        ,   ".."        =>
            {
                if p.pop().is_none()
                {
                    break;
                }
            }
        ,   _       => { p.push( String::from( x ) ); }
        }
    }

    if p.len() == 0
    {
        return Err( error::ErrorForbidden( "" ) );
    }
    else
    {
        theme_content_impl( ctx, path::Path::new( p.join( "/" ).as_str() ) )
    }
}

///
async fn root( ctx : web::Data< Mutex< Context > > ) -> Result< afs::NamedFile >
{
    theme_content_impl( ctx, path::Path::new( "main.html" ) )
}


///
#[actix_rt::main]
async fn main() -> io::Result<()>
{
    std::env::set_var( "RUST_LOG", "debug" );
    env_logger::init();

    let config = get_config();

    if config.is_none()
    {
        return Err( std::io::Error::new( std::io::ErrorKind::Other, "stop!" ) );
    }

    let ( mpdcom_tx,    mpdcom_rx )     = mpsc::channel::< mpdcom::MpdComRequest >( 128 );
    let ( mpdfifo_tx,   mpdfifo_rx )    = mpsc::channel::< mpdfifo::MpdFifoRequest >( 2 );

    let ctx =
        web::Data::new(
            Mutex::new(
                Context
                {
                    config              : config.unwrap()
                ,   mpdcom_tx           : mpdcom_tx
                ,   mpd_status          : Ok( mpdcom::MpdComOk::new() )
                ,   status_ws_sessions  : HashMap::new()
                ,   mpdfifo_tx          : mpdfifo_tx
                ,   bar_dat_json        : String::new()
                ,   bar_cap_json        : String::new()
                }
            )
        );

    let bind_addr =
    {
        String::from( &ctx.lock().unwrap().config.bind_addr )
    };

    let ctx_t = ctx.clone();

    actix_rt::spawn(
        async
        {
            log::debug!( "mpdcom starting." );

            mpdcom::mpdcom_task( ctx_t, mpdcom_rx ).await.ok();

            log::debug!( "mpdcom stop." );
        }
    );

    log::debug!( "httpserver stating." );

    let ctx_t = ctx.clone();

    let server = HttpServer::new( move ||
        {
            App::new()
            .app_data( ctx_t.clone() )
            .wrap( middleware::Logger::default() )
            .service(
                web::resource( "/status_ws" )
                    .route( web::get().to( status_ws ) )
            )
            .service(
                web::resource( "/status" )
                    .route( web::get().to( status ) )
                    .route( web::post().to( status ) )
            )
            .service(
                web::resource( "/cmd" )
                    .route( web::get().to( cmd_get ) )
                    .route( web::post().to( cmd_post ) )
            )
            .service(
                web::resource( "/favicon.ico/*" )
                    .to( favicon )
            )
            .service(
                web::resource( "/theme/*" )
                    .to( theme )
            )
            .service(
                web::resource( "/" )
                    .to( root )
            )
        }
    )
    .bind( bind_addr )?
    .run()
    .await;

    {
        let ( mut req, rx ) = mpdcom::MpdComRequest::new();

        req.req = mpdcom::MpdComRequestType::Shutdown;

        &ctx.lock().unwrap().mpdcom_tx.send( req ).await;

        rx.await.ok();

        log::debug!( "mpdcom shutdown." );
    }

    {
        let ( mut req, rx ) = mpdfifo::MpdFifoRequest::new();

        req.req = mpdfifo::MpdFifoRequestType::Shutdown;

        &ctx.lock().unwrap().mpdfifo_tx.send( req ).await;

        rx.await.ok();

        log::debug!( "mpdfifo shutdown." );
    }

    server
}

