//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///
use std::io;
use std::io::prelude::*;
use std::path;
use std::fs;
use std::net::{ SocketAddr, ToSocketAddrs };

use serde::{ Deserialize, Serialize };

pub const THEME_MAIN        : &str = "main.html";
pub const THEME_DIR         : &str = "_theme";
pub const THEME_DEFAULT_DIR : &str = "_default";
pub const THEME_COMMON_DIR  : &str = "_common";
pub const THEME_HIDE_DIR    : &str = "^[_.]";

///
#[derive(Debug, Deserialize, Clone)]
pub struct Config
{
    pub config_dyn      : String
,       bind_addr       : String
,       mpd_addr        : String
,   pub mpd_protolog    : bool
,   pub mpd_fifo        : String
,   pub mpd_fifo_fftmode: u32
,   pub log_level       : String
,   pub theme_dir       : String
}


impl Config
{
    pub fn bind_addr( &self ) -> SocketAddr
    {
        self.bind_addr.to_socket_addrs().unwrap().next().unwrap()
    }

    pub fn mpd_addr( &self ) -> SocketAddr
    {
        self.mpd_addr.to_socket_addrs().unwrap().next().unwrap()
    }
}

///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigDyn
{
    pub theme           : String
,   pub spec_delay      : u32
}

///
impl ConfigDyn
{
    pub fn new() -> ConfigDyn
    {
        ConfigDyn
        {
            theme           : String::from( THEME_DEFAULT_DIR )
        ,   spec_delay      : 500
        }
    }

    pub fn update( &mut self, newval : &str )
    {
        let newval : serde_json::Result< ConfigDynInput > = serde_json::from_str( newval );

        match newval
        {
            Ok( nv ) =>
            {
                if let Some( x ) = nv.theme
                {
                    log::debug!( "update dyn theme {}", &x );
                    self.theme = String::from( &x );
                }

                if let Some( x ) = nv.spec_delay
                {
                    log::debug!( "update dyn spec_delay {}", x );
                    self.spec_delay = x;
                }
            }
        ,   Err( x ) =>
            {
                log::error!( "{:?}", x );
            }
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ConfigDynOutput
{
    pub theme       : String
,   pub themes      : Vec< String >
,   pub spec_delay  : u32
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigDynInput
{
    pub theme      : Option< String >
,   pub spec_delay : Option< u32 >
}

pub fn make_config_dyn_output( config : &Config, config_dyn : &ConfigDyn ) -> ConfigDynOutput
{
    let mut path = path::PathBuf::new();

    if config.theme_dir != ""
    {
        path.push( &config.theme_dir );
    }
    else
    {
        path.push( THEME_DIR );
    }

    let mut themes = Vec::< String >::new();

    themes.push( String::from( THEME_DEFAULT_DIR ) );

    if let Ok( entries ) = fs::read_dir( path )
    {
        for entry in entries
        {
            if let Ok( entry ) = entry
            {
                if let Ok( entry_s ) = entry.file_name().into_string()
                {
                    lazy_static!
                    {
                        static ref RE : regex::Regex =
                            regex::Regex::new( THEME_HIDE_DIR ).unwrap();
                    }

                    if !RE.is_match( &entry_s )
                    {
                        let mut path_main = entry.path();

                        path_main.push( THEME_MAIN );

                        if path_main.is_file()
                        {
                            themes.push( String::from( &entry_s ) );
                        }
                    }
                }
            }
        }
    }

    ConfigDynOutput
    {
        theme           : String::from( &config_dyn.theme )
    ,   themes          : themes
    ,   spec_delay      : config_dyn.spec_delay
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
