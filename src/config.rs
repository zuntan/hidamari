//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io;
use std::io::prelude::*;
use std::fs;
use std::net::ToSocketAddrs;

use tokio::time::{ Duration };

use serde::{ Serialize, Deserialize };

///
#[derive(Debug, Deserialize, Clone)]
pub struct Config
{
    pub config_dyn  : String
,   pub bind_addr   : String
,   pub mpd_addr    : String
,   pub mpd_protolog: bool
,   pub mpd_fifo    : String
,   pub log_level   : String
,   pub theme_dir   : String
}

///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigDyn
{
    pub theme           : String
,   pub mpdfifo_delay   : Duration
}

///
impl ConfigDyn
{
    pub fn new() -> ConfigDyn
    {
        ConfigDyn
        {
            theme           : String::from( "_default" )
        ,   mpdfifo_delay   : Duration::from_millis( 500 )
        }
    }
}

///
pub fn get_config() -> Option< Config >
{

    let err_prefix = "config load error ";

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

        match fs::File::open( &t ).map( |f| io::BufReader::new( f ) )
        {
            Ok( mut f ) =>
            {
                log::info!( "config loading [{}]", &t );

                let mut tmp_conts = String::new();

                match f.read_to_string( &mut tmp_conts )
                {
                    Ok( _ ) =>
                    {
                        conts = Some( tmp_conts );
                        break;
                    }
                ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
                }
            }
        ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
        }
    }

    if conts.is_some()
    {
        match toml::de::from_str::<Config>( &conts.unwrap() )
        {
            Ok( x ) =>
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
        ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
        }
    }

    log::error!( "config load error." );
    None
}

///
pub fn get_config_dyn( config : &Config ) -> ConfigDyn
{
    let err_prefix = "config dyn load error ";

    let t = &config.config_dyn;

    if t != ""
    {
        log::debug!( "config dyn try loading [{:?}]", t );

        match fs::File::open( t ).map( |f| io::BufReader::new( f ) )
        {
            Ok( mut f ) =>
            {
                let mut conts = String::new();

                match f.read_to_string( &mut conts )
                {
                    Ok( _ ) =>
                    {
                        match toml::de::from_str::<ConfigDyn>( &conts )
                        {
                            Ok( x ) =>
                            {
                                log::info!( "config dyn loaded. [{:?}]", t );
                                return x;
                            }
                        ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
                        }
                    }
                ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
                }
            }
        ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
        }
    }

    log::info!( "config dyn load canceled." );
    ConfigDyn::new()
}


pub fn save_config_dyn( config : &Config, config_dyn : &ConfigDyn )
{
    let err_prefix = "config dyn save error";

    let t = &config.config_dyn;

    if t != ""
    {
        log::debug!( "config dyn try saving [{:?}]", t );

        match toml::ser::to_string_pretty( config_dyn )
        {
            Ok( x ) =>
            {
                match fs::File::create( t ).map( |f| io::BufWriter::new( f ) )
                {
                    Ok( mut f ) =>
                    {
                        if let Err( x ) = f.write( x.as_bytes() )
                        {
                            log::error!( "{} [{:?}]", err_prefix, &x );
                        }
                        else
                        {
                            log::info!( "config dyn saved. [{:?}]", t );
                        }
                    }
                ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
                }
            }
        ,   Err( x ) => { log::error!( "{} [{:?}]", err_prefix, &x ); }
        }
    }
    else
    {
        log::warn!( "config dyn save canceled." );
    }
}
