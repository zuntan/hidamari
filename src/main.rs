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
use std::sync::Mutex;

// use actix_web::http::{ header, Method, StatusCode };

use actix_web::{ error, /* guard, */ middleware, web };
use actix_web::{ App, HttpRequest, HttpResponse, HttpServer, Result };
use actix_files as afs;

use tokio::sync::{ oneshot, mpsc };

use chrono::prelude::*;

use serde::{ /* Serialize, */ Deserialize };

mod mpdcom;

///
#[derive(Debug, Deserialize, Clone)]
struct Config
{
    bind_addr   : String
,   mpd_addr    : String
,   mpd_protolog: bool
,   log_level   : String
,   theme_dir   : String
,   theme       : String
}

///
pub struct Context
{
    config          : Config
,   mpdcom_tx       : mpsc::Sender< mpdcom::MpdComRequest >
,   mpd_status_time : Option< chrono::DateTime<Local> >
,   mpd_status      : Vec<(String, String)>
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
async fn status( ctx : web::Data< Mutex< Context > > ) -> HttpResponse
{
    let ctx = ctx.lock().unwrap();

    if ctx.mpd_status_time.is_some()
    {
        HttpResponse::Ok().json( Result::<_,()>::Ok( &ctx.mpd_status ) )
    }
    else
    {
        HttpResponse::Ok().json( Result::<(),_>::Err( "" ) )
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
    fn to_request( &self ) -> ( mpdcom::MpdComRequest, oneshot::Receiver< mpdcom::MpdComResult > )
    {
        let ( mut req, rx ) = mpdcom::MpdComRequest::new();

        req.req += &self.cmd;

        if self.arg1.is_some()
        {
            req.req += " ";
            req.req += &mpdcom::quote_arg( self.arg1.as_ref().unwrap().as_str() );
        }

        if self.arg2.is_some()
        {
            req.req += " ";
            req.req += &mpdcom::quote_arg( self.arg2.as_ref().unwrap().as_str() );
        }

        if self.arg3.is_some()
        {
            req.req += " ";
            req.req += &mpdcom::quote_arg( self.arg3.as_ref().unwrap().as_str() );
        }

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

    let ( tx, rx ) = mpsc::channel::< mpdcom::MpdComRequest >( 128 );

    let ctx = web::Data::new( Mutex::new(
        Context
        {
            config          : config.unwrap()
        ,   mpdcom_tx       : tx
        ,   mpd_status_time : None
        ,   mpd_status      : Vec::new()
        }
    ) ) ;

    let bind_addr = {
        String::from( &ctx.lock().unwrap().config.bind_addr )
    };

    let ctx_t = ctx.clone();

    actix_rt::spawn(
        async
        {
            log::debug!( "mpdcom starting." );

            mpdcom::mpdcom_task( ctx_t, rx ).await.ok();

            log::debug!( "mpdcom stop." );
        }
    );

    log::debug!( "httpserver stating." );

    let ctx_t = ctx.clone();

    let server = HttpServer::new( move || {
        App::new()
            .app_data( ctx_t.clone() )
            .wrap( middleware::Logger::default() )
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

        req.req = String::from( "close" );

        &ctx.lock().unwrap().mpdcom_tx.send( req ).await;

        rx.await.ok();

        log::debug!( "mpdcom shutdown." );
    }

    server
}

