//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///
use std::io;
use std::io::prelude::*;
use std::path::{ PathBuf };
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;
use std::net::{ SocketAddr, ToSocketAddrs };

use tokio::sync;
use tokio::time;

use url::Url;

use serde::{ Deserialize, Serialize };

use crate::event;
use crate::mpdcom;

pub const CONTENTS_DIR      : &str = "_contents";
pub const THEME_DIR         : &str = "theme";
pub const THEME_MAIN        : &str = "main.html";
pub const THEME_FAVICON_ICO : &str = "favicon.ico";
pub const THEME_DEFAULT_DIR : &str = "_default";
pub const THEME_COMMON_DIR  : &str = "_common";
pub const THEME_HIDE_DIR    : &str = "^[_.]";

pub const SOUNDS_DIR        : &str = "sounds";
pub const TESTSOUNDS_NAME   : &str = r"^441-[12]-16-\d+s-(.+).mp3";

pub const SOUNDS_URL_PATH   : &str = "sounds";

///
#[derive(Debug, Deserialize, Clone)]
pub struct Config
{
    pub config_dyn          : String
,       bind_addr           : String
,       mpd_addr            : String
,       self_url_for_mpd    : String
,   pub mpd_protolog        : bool
,   pub mpd_fifo            : String
,   pub mpd_fifo_fftmode    : u32
,   pub log_level           : String
,   pub contents_dir        : String
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

    pub fn self_url_for_mpd( &self ) -> Option< String >
    {
        let url =
        {
            if self.self_url_for_mpd == ""
            {
                String::from( "http://127.0.0.1" )
            }
            else
            {
                String::from( &self.self_url_for_mpd )
            }
        };

        let mut url = match Url::parse( &url )
        {
            Ok(x) => x
        ,   Err(x) => {
                log::error!( "URL ParseError {:?}", x );
                return None;
            }
        };

        if url.port().is_none()
        {
            let bind_port =
                match self.bind_addr()
                {
                    SocketAddr::V4( x ) => { x.port() }
                ,   SocketAddr::V6( x ) => { x.port() }
                };

            match url.port_or_known_default()
            {
                Some( port ) =>
                {
                    if port != bind_port
                    {
                        url.set_port( Some( bind_port ) ).ok();
                    }
                }
            ,   None =>
                {
                    url.set_port( Some( bind_port ) ).ok();
                }
            }
        }

        url.set_fragment( None );
        url.set_path( "" );
        url.set_query( None );

        Some( url.into_string() )
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

pub struct WsSession
{
  pub   ws_sig  : String
, pub   ev_tx   : event::EventSender
}

pub type WsSessions = HashMap< u64, WsSession >;

pub struct Context
{
  pub   config          : Config
, pub   config_dyn      : ConfigDyn

, pub   mpdcom_tx       : sync::mpsc::Sender< mpdcom::MpdComRequest >
, pub   mpd_status_json : String

, pub   mpd_volume      : u8
, pub   mpd_mute        : bool

, pub   spec_enable     : bool
, pub   spec_data_json  : String
, pub   spec_head_json  : String

, pub   ws_sess_stop    : bool
, pub   ws_sess_no      : u64
, pub   ws_sessions     : WsSessions

, pub   ws_status_intv  : time::Duration
, pub   ws_data_intv    : time::Duration
, pub   ws_send_intv    : time::Duration

, pub   product         : String
, pub   version         : String
}

impl Context
{
    pub fn new(
        config      : Config
    ,   config_dyn  : ConfigDyn
    ,   mpdcom_tx   : sync::mpsc::Sender< mpdcom::MpdComRequest >
    ,   product     : &str
    ,   version     : &str
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
        ,   spec_enable     : false
        ,   spec_data_json  : String::new()
        ,   spec_head_json  : String::new()
        ,   ws_sess_stop    : false
        ,   ws_sess_no      : 0
        ,   ws_sessions     : WsSessions::new()
        ,   ws_status_intv  : time::Duration::from_millis( 200 )
        ,   ws_data_intv    : time::Duration::from_millis( 200 )
        ,   ws_send_intv    : time::Duration::from_secs( 3 )
        ,   product         : String::from( product )
        ,   version         : String::from( version )
        }
    }

    pub fn get_contents_path( &self ) -> PathBuf
    {
        let mut path = PathBuf::new();

        if self.config.contents_dir != ""
        {
            path.push( &self.config.contents_dir );
        }
        else
        {
            path.push( CONTENTS_DIR );
        }

        path
    }

    pub fn get_theme_path( &self ) -> PathBuf
    {
        let mut path = self.get_contents_path();

        path.push( THEME_DIR );

        if self.config_dyn.theme != ""
        {
            path.push( &self.config_dyn.theme );
        }

        path
    }

    pub fn get_common_path( &self ) -> PathBuf
    {
        let mut path = self.get_contents_path();

        path.push( THEME_DIR );
        path.push( THEME_COMMON_DIR );

        path
    }

    pub fn get_sounds_path( &self ) -> PathBuf
    {
        let mut path = self.get_contents_path();

        path.push( SOUNDS_DIR );

        path
    }

    pub fn make_config_dyn_output( &self ) -> ConfigDynOutput
    {
        let mut path = self.get_contents_path();

        path.push( THEME_DIR );

        let mut themes = Vec::< String >::new();

        themes.push( String::from( THEME_DEFAULT_DIR ) );

        if let Ok( entries ) = fs::read_dir( path )
        {
            for entry in entries
            {
                if let Ok( entry ) = entry
                {
                    if let Ok( entry_fn ) = entry.file_name().into_string()
                    {
                        lazy_static!
                        {
                            static ref RE : regex::Regex =
                                regex::Regex::new( THEME_HIDE_DIR ).unwrap();
                        }

                        if !RE.is_match( &entry_fn )
                        {
                            let mut path_main = entry.path();

                            path_main.push( THEME_MAIN );

                            if path_main.is_file()
                            {
                                themes.push( String::from( &entry_fn ) );
                            }
                        }
                    }
                }
            }
        }

        ConfigDynOutput
        {
            theme           : String::from( &self.config_dyn.theme )
        ,   themes          : themes
        ,   spec_delay      : self.config_dyn.spec_delay
        }
    }

    pub fn testsound_url( &self ) -> Vec< String >
    {
        if let Some( self_url_for_mpd ) = self.config.self_url_for_mpd()
        {
            let mut path = self.get_contents_path();

            path.push( SOUNDS_DIR );

            let mut ts = Vec::< String >::new();

            if let Ok( entries ) = fs::read_dir( path )
            {
                for entry in entries
                {
                    if let Ok( entry ) = entry
                    {
                        if let Ok( entry_fn ) = entry.file_name().into_string()
                        {
                            lazy_static!
                            {
                                static ref RE : regex::Regex =
                                    regex::Regex::new( TESTSOUNDS_NAME ).unwrap();
                            }

                            if RE.is_match( &entry_fn )
                            {
                                ts.push( format!( "{}{}/{}", &self_url_for_mpd, SOUNDS_URL_PATH, entry_fn ) );
                            }
                        }
                    }
                }
            }

            ts.sort();
            ts
        }
        else
        {
            Vec::< String >::new()
        }
    }
}

///
pub type ARWLContext = Arc< sync::RwLock< Context > >;

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
