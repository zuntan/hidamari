#[macro_use]
extern crate actix_web;
extern crate actix_utils;
extern crate actix_rt;
extern crate tokio;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
use chrono::prelude::*;

use std::io;
use std::io::prelude::*;
use std::path;
use std::fs;
use log::{error, warn, info, debug};

use std::sync::{ Arc, Mutex };
use std::sync::mpsc;
use std::rc::Rc;
use std::cell::{ RefCell };

use actix_web::http::{ header, Method, StatusCode };
use actix_web::{ error, guard, middleware, web };
use actix_web::{ App, Error, HttpRequest, HttpResponse, HttpServer, Result };
use actix_files as afs;

use tokio::time::{ Duration, Instant };

use json::JsonValue;
use serde::{ Serialize, Deserialize };

mod dispatch;
mod mdpcom;

///
#[derive(Debug, Deserialize, Clone)]
struct Config
{
	bind_addr 	: String
,	mpd_addr  	: String
,	mpd_protolog: bool
,	log_level 	: String
,	theme_dir	: String
,	theme    	: String
}

struct ThreadResult
{
	msg	: String
}

struct ThreadCommand
{

	cmd	: String
,	tx	: mpsc::Sender<ThreadResult>
}

struct Context
{
	config			: Config
,	thread_tx		: mpsc::Sender<ThreadCommand>
,	mpd_status_time : Option< Instant >
,	mpd_status		: Vec<(String, String)>
}

impl Context
{
	fn get_theme_path( &self ) -> path::PathBuf
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

fn command_impl( ctx : web::Data< Mutex< Context > >, param : &CommandParam ) -> Result<HttpResponse>
{
	debug!("{:?}", &param );

	let ( tx, rx ) = mpsc::channel::<ThreadResult>();

	let cmd = ThreadCommand{ cmd : String::from( &param.cmd ), tx };

	ctx.lock().unwrap().thread_tx.send( cmd ).unwrap();

	let ret = rx.recv().unwrap();

	Ok( HttpResponse::Ok().json( &ret.msg ) )
}

///
fn get_config() -> Option< Config >
{
	let mut targets = vec![
		String::from( "hidamari.conf" )
	,	String::from( "/etc/hidamari.conf" )
	];

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2
    {
    	targets.insert( 0, args[1].clone() );
    }

    for t in targets
    {
		debug!( "config try loading [{:?}]", &t );

		if let Ok( mut f ) = fs::File::open( &t ).map( |f| io::BufReader::new( f ) )
		{
		    let mut conts = String::new();

			if let Ok( _ ) = f.read_to_string( &mut conts )
			{
				if let Ok( x ) = toml::de::from_str::<Config>( &conts )
				{
					info!( "config load [{}]", &t );
					return Some( x );
				}
			}
		}
	}

	error!( "config load error." );
    None
}

///
fn theme_content_impl( ctx : web::Data< Mutex< Context > >, p : &path::Path ) -> Result< afs::NamedFile >
{
	let mut path = ctx.lock().unwrap().get_theme_path();

	path.push( p );

	debug!("{:?}", &path );

	if path.is_file()
	{
	    Ok( afs::NamedFile::open( path )? )
	}
	else
	{
		return Err( error::ErrorNotFound( "" ) );
	}
}

#[derive(Debug, Deserialize, Clone)]
struct CommandParam
{
	cmd  : String
,	arg1 : Option<String>
,	arg2 : Option<String>
}

///
async fn command_get( ctx : web::Data< Mutex< Context > >, param: web::Query<CommandParam> ) -> Result<HttpResponse>
{
	debug!("command_get" );

	command_impl( ctx, &*param )
}

///
async fn command_post( ctx : web::Data< Mutex< Context > >, param: web::Form<CommandParam> ) -> Result<HttpResponse>
{
	debug!("command_post" );

	command_impl( ctx, &*param )
}


///
async fn favicon( ctx : web::Data< Mutex< Context > > ) -> Result< afs::NamedFile >
{
	theme_content_impl( ctx, path::Path::new( "favicon.ico" ) )
}

///
async fn theme( ctx : web::Data< Mutex< Context > >, req : HttpRequest ) -> Result< afs::NamedFile >
{
	debug!("{}", req.path() );

	let mut p : Vec<String> = Vec::new();

	for x in req.path().split( '/' ).skip(2)
	{
		match x
		{
			"" | "." 	=> {}
		,	".." 		=>
			{
				if p.pop().is_none()
				{
					break;
				}
			}
		,	_		=> { p.push( String::from( x ) ); }
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

	let ( tx, rx ) = mpsc::channel::<ThreadCommand>();

	let ctx = web::Data::new( Mutex::new(
		Context
		{
			config			: config.unwrap()
		,	thread_tx		: tx
		,	mpd_status_time : None
		,	mpd_status		: Vec::new()
		}
	) ) ;

	let bind_addr = {
		String::from( &ctx.lock().unwrap().config.bind_addr )
	};

	let ctx_t = ctx.clone();

	actix_rt::spawn( mpdcom::modComTask( ctx.clone(), rx ) );

    HttpServer::new( move || {
        App::new()
        	.app_data( ctx.clone() )
            .wrap( middleware::Logger::default() )
            .service(
            	web::resource( "/cmd" )
            		.route( web::get().to( command_get ) )
            		.route( web::post().to( command_post ) )
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
    .await
}
