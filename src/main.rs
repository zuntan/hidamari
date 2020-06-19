#[macro_use]
extern crate actix_web;

use std::io;
use std::io::prelude::*;
use std::path;
use std::fs;
use log::{error, warn, info, debug};

use std::sync::{ Arc, Mutex };
use std::rc::Rc;
use std::cell::{ RefCell };

use actix_web::http::{ header, Method, StatusCode };
use actix_web::{ error, guard, middleware, web };
use actix_web::{ App, Error, HttpRequest, HttpResponse, HttpServer, Result };
use actix_files as afs;

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
,	log_level 	: String
,	theme_dir	: String
,	theme    	: String
}

impl Config
{
	fn get_theme_path( &self ) -> path::PathBuf
	{
		let mut path = path::PathBuf::new();

		if self.theme_dir != ""
		{
			path.push( &self.theme_dir );
		}
		else
		{
			path.push( "_theme" );
		}

		if self.theme != ""
		{
			path.push( &self.theme );
		}

		path
	}
}

struct Context
{
	config : Config
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
	let mut path = ctx.lock().unwrap().config.get_theme_path();

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
	arg1 : Option<String>
,	arg2 : Option<String>
}

///
async fn command_get( ctx : web::Data< Mutex< Context > >, cmd: web::Path<String>, param: web::Query<CommandParam> ) -> Result<HttpResponse>
{
	debug!("command_get" );
	debug!("{:?}", &cmd );
	debug!("{:?}", &param );

	Ok( HttpResponse::Ok().json( "" ) )
}

///
async fn command_post( ctx : web::Data< Mutex< Context > >, cmd: web::Path<String>, param: web::Form<CommandParam> ) -> Result<HttpResponse>
{
	debug!("command_post" );
	debug!("{:?}", &cmd );
	debug!("{:?}", &param );

	Ok( HttpResponse::Ok().json( "" ) )
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

	let ctx = web::Data::new( Mutex::new(
		Context
		{
			config : config.unwrap()
		}
	) ) ;

	{
		let config = &ctx.lock().unwrap().config;
	}

	let bind_addr = {
		String::from( &ctx.lock().unwrap().config.bind_addr )
	};

	let ctxc = ctx.clone();

    HttpServer::new( move || {
        App::new()
        	.app_data( ctx.clone() )
            .wrap( middleware::Logger::default() )
            .service(
            	web::resource( "/cmd/{command}" )
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
