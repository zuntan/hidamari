//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///

extern crate pretty_env_logger;
extern crate tokio;
extern crate chrono;


#[macro_use]
extern crate lazy_static;

use std::sync::Arc;

use tokio::signal;
use tokio::sync;
use tokio::task;
use tokio::join;

use warp::Filter;

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
        }
    }
}

type ARWLContext = Arc< sync::RwLock< Context > >;

#[tokio::main]
async fn main() -> std::io::Result< () >
{
    std::env::set_var( "RUST_LOG", "debug" );

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

    let ( mut mpdfifo_tx,   mpdfifo_rx )    = event::make_channel();
    let arwlctx_c = arwlctx.clone();
    let h_mpdfifo : task::JoinHandle< _ > = task::spawn( mpdfifo::mpdfifo_task( arwlctx_c, mpdfifo_rx ) );

    log::debug!( "http server init." );

    let ( tx, rx ) = sync::oneshot::channel();

    let routes = warp::any().map( || "Hello, World!" );

    let ( addr, server ) =
        warp::serve( routes )
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
