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
use std::path;
use std::sync::{ Mutex };
use std::collections::hash_map::{ HashMap };

// use actix_web::http::{ header, Method, StatusCode };
use actix_web::{ error, /* guard, */ middleware, web };
use actix_web::{ App, HttpRequest, HttpResponse, HttpServer, Result, Error };
use actix_files as afs;
use actix_web_actors::ws;

use tokio::sync::{ oneshot, mpsc };

use serde::{ /* Serialize, */ Deserialize };

mod config;
mod mpdcom;
mod wssession;
mod mpdfifo;
mod event;
mod task;

use config::{ Config, ConfigDyn, get_config, get_config_dyn, save_config_dyn };

///
pub struct Context
{
    config              : Config
,   config_dyn          : ConfigDyn

,   mpdcom_tx           : mpsc::Sender< mpdcom::MpdComRequest >
,   mpd_status          : mpdcom::MpdComResult
,   status_ws_sessions  : wssession::WsSessions

,   spec_data_json      : String
,   spec_head_json      : String
}

impl Context
{
    fn new(
        config              : Config
    ,   config_dyn          : ConfigDyn
    ,   mpdcom_tx           : mpsc::Sender< mpdcom::MpdComRequest >
    ) -> Context
    {
        Context
        {
            config
        ,   config_dyn
        ,   mpdcom_tx
        ,   mpd_status          : Ok( mpdcom::MpdComOk::new() )
        ,   status_ws_sessions  : HashMap::new()
        ,   spec_data_json      : String::new()
        ,   spec_head_json      : String::new()
        }
    }
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

        if self.config_dyn.theme != ""
        {
            path.push( &self.config_dyn.theme );
        }

        path
    }
}


///
async fn ws( ctx : web::Data< Mutex< Context > >, r: HttpRequest, stream: web::Payload ) -> Result< HttpResponse, Error >
{
    ws::start( wssession::ArcWsSession::new( &ctx ), &r, stream )
}

///
async fn spec_data( ctx : web::Data< Mutex< Context > > ) -> HttpResponse
{
    let ctx = ctx.lock().unwrap();

    HttpResponse::Ok().body( &ctx.spec_data_json )
}

///
async fn spec_head( ctx : web::Data< Mutex< Context > > ) -> HttpResponse
{
    let ctx = ctx.lock().unwrap();

    HttpResponse::Ok().body( &ctx.spec_head_json )
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
async fn favicon( ctx : web::Data< Mutex< Context > > ) -> Result< afs::NamedFile >
{
    theme_content_impl( ctx, path::Path::new( "favicon.ico" ) )
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

    let config = config.unwrap();

    let config_dyn = get_config_dyn( &config );

    let ( mpdcom_tx,    mpdcom_rx )     = mpsc::channel::< mpdcom::MpdComRequest >( 128 );

    let ctx =
        web::Data::new(
            Mutex::new(
                Context::new( config, config_dyn, mpdcom_tx )
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


    let ( mpdfifo_tx,   mpdfifo_rx )    = event::make_channel();
    let ctx_t = ctx.clone();

    actix_rt::spawn(
        async
        {
            log::debug!( "mpdfifo starting." );
            mpdfifo::mpdfifo_task( ctx_t, mpdfifo_rx ).await.ok();
            log::debug!( "mpdfifo stop." );
        }
    );

    let ( sart_tx, sart_rx )    = event::make_channel();
    let ctx_t = ctx.clone();

    actix_rt::spawn(
        async
        {
            log::debug!( "spectrum_responce_task starting." );
            task::spectrum_responce_task( ctx_t, sart_rx ).await.ok();
            log::debug!( "spectrum_responce_task stop." );
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
                web::resource( "/spec_data" )
                    .route( web::get().to( spec_data ) )
                    .route( web::post().to( spec_data ) )
            )
            .service(
                web::resource( "/spec_head" )
                    .route( web::get().to( spec_head ) )
                    .route( web::post().to( spec_head ) )
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
            .service(
                web::resource( "/ws" )
                    .route( web::get().to( ws ) )
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

    for mut tx in vec![ mpdfifo_tx, sart_tx ]
    {
        let ( mut req, rx ) = event::new_request();

        req.req = event::EventRequestType::Shutdown;

        tx.send( req ).await.ok();

        rx.await.ok();
    }

    {
        let ctx = &ctx.lock().unwrap();
        save_config_dyn( &ctx.config, &ctx.config_dyn );
    }


    server
}

