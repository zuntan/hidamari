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
use tokio::task;
use tokio::join;
use tokio::fs::File;
use tokio_util::codec::{ BytesCodec, FramedRead };

use warp::{ Filter, filters, reply::Reply, reply::Response, reject::Rejection, hyper::Body };

use headers::HeaderMapExt;

use serde::de::DeserializeOwned;

mod config;
mod mpdcom;
mod mpdfifo;
mod event;

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

,	version			: String
}

impl Context
{
    fn new(
        config      : config::Config
    ,   config_dyn  : config::ConfigDyn
    ,   mpdcom_tx   : sync::mpsc::Sender< mpdcom::MpdComRequest >
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
        ,	version			: String::new()
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

	        return RespResult::Ok( resp );
	    }
	,	Err( x ) =>
		{
			log::warn!( "{:?}", x );
		}
	}

    RespResult::Err( warp::reject::not_found() )
}

fn check_path( path : &str )
	-> Result< String, Rejection >
{
    let mut p = Vec::< String >::new();

    for x in path.split( '/' )
    {
        match x
        {
            "\\"    	=> { return Err( warp::reject::not_found() ); }
        ,   "" | "."    => {}
        ,   ".."        =>
            {
                if p.pop().is_none()
                {
                    return Err( warp::reject::not_found() );
                }
            }
        ,   _       	=> { p.push( String::from( x ) ); }
        }
    }

    Ok( p.join( "/" ) )
}

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


fn make_route_test()
	-> impl Filter< Extract = ( impl Reply, ), Error = Rejection > + Copy
{
	let route = make_route_getpost::< HashMap< String, String > >();

    let route = route.map( move | dic : HashMap< String, String > |
		{
			let mut ret = String::new();

			for ( k, v ) in dic
			{
				ret += &format!( "{} = {} \n", k, v );
			}

			ret
		}
	);

	route
}

async fn make_theme_file_response( arwlctx : ARWLContext, path: &str, is_common : bool, do_unshift : bool )
	-> RespResult
{
	let path =
	{
		if do_unshift
		{
			path.split( '/' )
				.skip(1)
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
	,	Ok( path ) =>
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
fn make_route( arwlctx : ARWLContext )
	-> filters::BoxedFilter< ( impl Reply, ) >
{
	let x_arwlctx = arwlctx.clone();

    let r_root =
    	warp::path::end()
    	.and( warp::any().map( move || x_arwlctx.clone() ) )
    	.and_then( | arwlctx : ARWLContext  | async move
			{
				make_theme_file_response( arwlctx, config::THEME_MAIN, false, false ).await
			}
    	);

    let x_arwlctx = arwlctx.clone();

    let r_favicon	= warp::path!( "favicon.ico" )
        .and( warp::any().map( move || x_arwlctx.clone() ) )
    	.and_then( | arwlctx : ARWLContext  | async move
			{
				make_theme_file_response( arwlctx, "favicon.ico", false, false ).await
			}
    	);

    let x_arwlctx = arwlctx.clone();

    let r_common = warp::path!( "common" / .. )
		.and( warp::path::full() )
		.and( warp::any().map( move || x_arwlctx.clone() ) )
		.and_then( | x : warp::path::FullPath, arwlctx : ARWLContext  | async move
			{
				make_theme_file_response( arwlctx, x.as_str(), true, true ).await
			}
		);

    let x_arwlctx = arwlctx.clone();

    let r_theme =
    	warp::path!( "theme" / .. )
		.and( warp::path::full() )
		.and( warp::any().map( move || x_arwlctx.clone() ) )
		.and_then( | x : warp::path::FullPath, arwlctx : ARWLContext  | async move
			{
				make_theme_file_response( arwlctx, x.as_str(), false, true ).await
			}
		);

    let x_arwlctx = arwlctx.clone();

    let r_cmd  = warp::path!( "cmd" )
    	.and( make_route_getpost::< HashMap< String, String > >() )
		.and( warp::any().map( move || x_arwlctx.clone() ) )
		.and_then( | dic : HashMap< String, String >, arwlctx : ARWLContext  | async move
			{
				StrResult::Ok( String::new() )
				/*
				RespResult::Ok( Response::new( Body::empty() ) )
				*/
    		}
    	);


    let r_test = warp::path!( "test" ).and( make_route_test() );

    let routes = r_root
    			.or( r_favicon )
    			.or( r_theme )
    			.or( r_common )
				.or( r_cmd )
    			.or( r_test )
				;

	let with_server = warp::reply::with::header( "server", "hidamari" );
    let with_log	= warp::log( "hidamari" );

    routes.with( with_server ).with( with_log ).boxed()
}

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
                Context::new( config, config_dyn, mpdcom_tx )
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

    let ( mut req, rx ) = event::new_request();
    req.req = event::EventRequestType::Shutdown;
    let _ = mpdfifo_tx.send( req ).await;
    let _ = join!( h_mpdfifo );
    log::info!( "mpdfifo_task shutdown." );

    let ( mut req, rx ) = mpdcom::MpdComRequest::new();
    req.req = mpdcom::MpdComRequestType::Shutdown;
    let _ = arwlctx.write().await.mpdcom_tx.send( req ).await;
    let _ = join!( h_mpdcom );
    log::info!( "mpdcom_task shutdown." );

    let ctx = arwlctx.read().await;
    config::save_config_dyn( &ctx.config, &ctx.config_dyn );

    Ok(())
}
