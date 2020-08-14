//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::{ HashMap, HashSet };
use std::fs;
use std::net::{ SocketAddr, ToSocketAddrs };

use rand::prelude::*;

use tokio::sync;
use tokio::time;

use url::Url;

use serde::{ Deserialize, Serialize };

use crate::event;
use crate::mpdcom;
use crate::mpdfifo;
use crate::asyncread;
use crate::btctrl;

pub const CONTENTS_DIR      : &str = "_contents";
pub const THEME_DIR         : &str = "theme";
pub const THEME_MAIN        : &str = "main.html";
pub const THEME_FAVICON_ICO : &str = "favicon.ico";
pub const THEME_DEFAULT_DIR : &str = "_default";
pub const THEME_COMMON_DIR  : &str = "_common";
pub const THEME_HIDE_DIR    : &str = "^[_.]";

pub const TESTSOUND_DIR     : &str = "tsource";
pub const TESTSOUND_NAME    : &str = r"^test-\d+-\d-\d+-\d+s-(.+).mp3";
//pub const TESTSOUND_NAME  : &str = r"^test-44100-1-16-10s-cord_a.mp3";

pub const TESTSOUND_URL_PATH        : &str = "tsource";
pub const HIDAMARI_EXT_SOURCE_PROTO : &str = "^([at]source)://";

pub const ALSA_SOURCE_PROTO         : &str = "asource://";
pub const MPD_SINK_PROTO            : &str = "mpdsink://";
pub const ALSA_SINK_PROTO           : &str = "asink://";

pub const HIDAMARI_EXT_SINK_MPD_PROTO   : &str = "^mpdsink://([0-9]+)";
pub const HIDAMARI_EXT_SINK_ALSA_PROTO  : &str = "^asink://";

//pub const HIDAMARI_MPD_PROXY_STREAM_NAME    : &str = "Proxy Stream";
pub const HIDAMARI_MPD_PROXY_STREAM_PATH    : &str = "/stream";

pub const IGNORE_CHECK_URL          : &str =  "^([at]source|alsa|mpdsink|asink)://";
pub const URL_WITH_NAME             : &str =  r"^\s*\[([^\]]+)\]\s*";

pub const MPD_USER_AGENT    : &str = r"Music Player Daemon (\d+.\d+.\d+)";

pub const _HEADER_SHOUTCAST_ICY_METADATA_KEY    : &str = "icy-metadata";
pub const _HEADER_SHOUTCAST_ICY_METADATA_VAL    : &str = "1";
pub const _HEADER_SHOUTCAST_ICY_NAME_KEY        : &str = "icy-name";
pub const _HEADER_SHOUTCAST_ICY_GENRE_KEY       : &str = "icy-genre";
pub const _HEADER_SHOUTCAST_ICY_URL_KEY         : &str = "icy-url";
pub const _HEADER_SHOUTCAST_ICY_BR_KEY          : &str = "icy-br";
pub const _HEADER_SHOUTCAST_ICY_PUB_KEY         : &str = "icy-pub";
pub const _HEADER_SHOUTCAST_ICY_PUB_VAL         : &str = "1";
pub const _HEADER_SHOUTCAST_ICY_DESC_KEY        : &str = "icy-description";


///
#[derive(Debug, Deserialize, Clone)]
pub struct Config
{
    pub config_dyn          : String
,       bind_addr           : String
,       mpd_addr            : String
,   pub mpd_httpd_url       : String
,       self_url_for_mpd    : String
,   pub mpd_protolog        : bool
,   pub mpd_fifo            : String
,   pub mpd_fifo_fftmode    : u32
,   pub contents_dir        : String
,   pub albumart_upnp       : bool
,   pub albumart_localdir   : String
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
    pub theme       : String
,   pub spec_delay  : u32
,   pub url_list    : Vec< String >
,   pub aux_in      : Vec< String >
}

///
impl ConfigDyn
{
    pub fn new() -> ConfigDyn
    {
        ConfigDyn
        {
            theme       : String::from( THEME_DEFAULT_DIR )
        ,   spec_delay  : 50
        ,   url_list    : Vec::< String >::new()
        ,   aux_in      : Vec::< String >::new()
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ConfigDynOutput
{
    pub theme       : String
,   pub themes      : Vec< String >
,   pub spec_delay  : u32
,   pub url_list    : Vec< String >
,   pub aux_in      : Vec< String >
}

#[derive(Debug, Serialize, Clone)]
pub struct ConfigDynOutputError
{
    pub err_msg     : String
}

pub type ConfigDynOutputResult = Result< ConfigDynOutput, ConfigDynOutputError >;

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigDynInput
{
    pub theme       : Option< String >
,   pub spec_delay  : Option< u32 >
,   pub url_list    : Option< Vec< String > >
,   pub aux_in      : Option< Vec< String > >
}

#[derive(Debug)]
pub struct WsSession
{
  pub   ws_sig  : String
, pub   ev_tx   : event::EventSender
}

pub type WsSessions = HashMap< u64, WsSession >;

#[derive(Debug)]
pub struct Context
{
  pub   config          : Config
, pub   config_dyn      : ConfigDyn

, pub   mpdcom_tx       : sync::mpsc::Sender< mpdcom::MpdComRequest >
, pub   mpd_status_json : String

, pub   mpd_volume      : u8
, pub   mpd_mute        : bool

, pub   mpdfifo_tx      : sync::mpsc::Sender< mpdfifo::MpdfifoRequest >

, pub   spec_enable     : bool
, pub   spec_data_json  : String
, pub   spec_head_json  : String

, pub   ws_sess_stop    : bool
, pub   ws_sess_no      : u64
, pub   ws_sessions     : WsSessions

, pub   ws_status_intv  : time::Duration
, pub   ws_data_intv    : time::Duration
, pub   ws_send_intv    : time::Duration

, pub   btctrl_tx       : sync::mpsc::Sender< btctrl::BtctrlRequest >
, pub   bt_status_json  : String
, pub   bt_notice_json  : String

, pub   notice_reply_token      : String
, pub   notice_reply_token_time : time::Instant
, pub   bt_agent_io_rx_opend    : bool
, pub   bt_agent_io_tx          : sync::mpsc::Sender< btctrl::BtctrlRepryType >

, pub   io_list_json    : String
, pub   sdf_list        : Vec< asyncread::WmShutdownFlag >
, pub   shutdown        : bool
, pub   rng             : StdRng

, pub   product         : String
, pub   version         : String
}

///
pub type ARWLContext = Arc< sync::RwLock< Context > >;

pub type UrlTitleList = Vec< ( String, String ) >;

impl Context
{
    pub fn new(
        config          : Config
    ,   config_dyn      : ConfigDyn
    ,   mpdcom_tx       : sync::mpsc::Sender< mpdcom::MpdComRequest >
    ,   mpdfifo_tx      : sync::mpsc::Sender< mpdfifo::MpdfifoRequest >
    ,   btctrl_tx       : sync::mpsc::Sender< btctrl::BtctrlRequest >
    ,   bt_agent_io_tx  : sync::mpsc::Sender< btctrl::BtctrlRepryType >
    ,   product         : &str
    ,   version         : &str
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
        ,   mpdfifo_tx
        ,   spec_enable     : false
        ,   spec_data_json  : String::new()
        ,   spec_head_json  : String::new()
        ,   ws_sess_stop    : false
        ,   ws_sess_no      : 0
        ,   ws_sessions     : WsSessions::new()
        ,   ws_status_intv  : time::Duration::from_millis( 200 )
        ,   ws_data_intv    : time::Duration::from_millis( 200 )
        ,   ws_send_intv    : time::Duration::from_secs( 3 )
        ,   btctrl_tx
        ,   bt_status_json  : String::new()
        ,   bt_notice_json  : String::new()

        ,   notice_reply_token      : String::new()
        ,   notice_reply_token_time : time::Instant::now()
        ,   bt_agent_io_rx_opend    : false
        ,   bt_agent_io_tx

        ,   io_list_json    : String::new()
        ,   sdf_list        : Vec::< asyncread::WmShutdownFlag >::new()
        ,   shutdown        : false
        ,   rng             : SeedableRng::from_rng( thread_rng() ).unwrap()

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

    pub fn get_tsound_path( &self ) -> PathBuf
    {
        let mut path = self.get_contents_path();

        path.push( TESTSOUND_DIR );

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
        ,   url_list        : self.config_dyn.url_list  .iter().map( | x | String::from( x ) ).collect()
        ,   aux_in          : self.config_dyn.aux_in    .iter().map( | x | String::from( x ) ).collect()
        }
    }

    pub fn update_config_dyn( &mut self, newval : &str ) -> Option< ConfigDynOutputError >
    {
        let newval : serde_json::Result< ConfigDynInput > = serde_json::from_str( newval );

        match newval
        {
            Ok( nv ) =>
            {
                // check

                if let Some( ref x ) = nv.url_list
                {
                    let ret = check_urls( x );

                    if ret.is_some()
                    {
                        return ret;
                    }
                }

                if let Some( ref x ) = nv.aux_in
                {
                    let ret = check_urls( x );

                    if ret.is_some()
                    {
                        return ret;
                    }
                }

                // update

                if let Some( x ) = nv.theme
                {
                    log::debug!( "update dyn theme {}", &x );
                    self.config_dyn.theme = String::from( &x );
                }

                if let Some( x ) = nv.spec_delay
                {
                    log::debug!( "update dyn spec_delay {}", x );
                    self.config_dyn.spec_delay = x;
                }

                if let Some( x ) = nv.url_list
                {
                    let x : Vec< String > =
                        x.iter()
                        .map( | x | split_url_with_name( x ) )
                        .map( | ( u, n ) | concat_url_with_name( &u, &n ) )
                        .collect()
                        ;

                    let x = make_uniq_list( &x );
                    log::debug!( "update dyn url_list {:?}", x );
                    self.config_dyn.url_list = x;
                }

                if let Some( x ) = nv.aux_in
                {
                    let x : Vec< String > =
                        x.iter()
                        .map( | x | split_url_with_name( x ) )
                        .map( | ( u, n ) | concat_url_with_name( &u, &n ) )
                        .collect()
                        ;

                    let x = make_uniq_list( &x );
                    log::debug!( "update dyn aux_in {:?}", x );
                    self.config_dyn.aux_in  = x;
                }
            }
        ,   Err( x ) =>
            {
                return Some(
                    ConfigDynOutputError
                    {
                        err_msg : String::from( format!( "{:?}", x ) )
                    }
                );
            }
        }

        None
    }

    pub fn append_url( &mut self, url : &str )
    {
        let url = String::from( url.trim() );

        if !self.config_dyn.url_list.contains( &url )
        {
            self.config_dyn.url_list.push( url );
        }
    }

    pub fn testsound_urllist( &self ) -> UrlTitleList
    {
        if let Some( self_url_for_mpd ) = self.config.self_url_for_mpd()
        {
            let mut path = self.get_contents_path();

            path.push( TESTSOUND_DIR );

            let mut ts = UrlTitleList::new();

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
                                    regex::Regex::new( TESTSOUND_NAME ).unwrap();
                            }

                            if RE.is_match( &entry_fn )
                            {
                                let mut title = String::new();

                                match mp3_metadata::read_from_file( entry.path() )
                                {
                                    Ok( mp3md ) =>
                                    {
                                        if let Some( x ) = mp3md.optional_info.iter().rev().find( |x| x.title.is_some() )
                                        {
                                            if let Some( ref x ) = x.title
                                            {
                                                title = String::from( x );
                                            }
                                        }

                                        if title == ""
                                        {
                                            if let Some( x ) = mp3md.tag
                                            {
                                                title = x.title;
                                            }
                                        }
                                        else
                                        {
                                            title = String::from( title.trim_matches( | c | c == ' ' || c == '\u{0}' ) );
                                        }
                                    }
                                ,   Err( x ) =>
                                    {
                                        log::error!( "mp3_metadata error {:?}", x );
                                    }
                                }

                                let entry = (
                                    format!( "{}{}/{}", &self_url_for_mpd, TESTSOUND_URL_PATH, entry_fn )
                                ,   title
                                );

                                log::debug!( "mp3_metadata {:?}", &entry );

                                ts.push( entry );
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
            UrlTitleList::new()
        }
    }

    pub fn update_hidamari_url( &self, url : &str ) -> String
    {
        if let Some( self_url_for_mpd ) = self.config.self_url_for_mpd()
        {
            lazy_static!
            {
                static ref RE : regex::Regex =
                    regex::Regex::new( HIDAMARI_EXT_SOURCE_PROTO ).unwrap();
            }

            match RE.captures( &url )
            {
                Some( cap ) =>
                {
                    let tail    = &url[ cap.get( 0 ).unwrap().end() .. ];
                    let proto   = cap.get( 1 ).unwrap().as_str();

                    return format!( "{}{}/{}", &self_url_for_mpd, proto, tail );
                }
            ,   None => {}
            }
        }

        String::from( url )
    }

    pub fn aux_in( &self ) -> Vec< String >
    {
        make_uniq_list( &self.config_dyn.aux_in )
    }

    pub fn sdf_add( &mut self, wsf : asyncread::WmShutdownFlag )
    {
        let n1 = self.sdf_list.len();

        let mut d = Vec::< usize >::new();

        for ( i, x ) in self.sdf_list.iter().enumerate()
        {
            if x.upgrade().is_none()
            {
                d.push( i );
            }
        }

        if !d.is_empty()
        {
            for i in d.iter().rev()
            {
                self.sdf_list.swap_remove( *i );
            }
        }

        self.sdf_list.push( wsf );

        let n2 = self.sdf_list.len();

        log::debug!( "sdf_add {} -> {}", n1, n2 );
    }

    pub fn sdf_shutdown( &self )
    {
        let mut c = 0;
        let     n = self.sdf_list.len();

        for x in self.sdf_list.iter()
        {
            if let Some( x ) = x.upgrade()
            {
                let mut f = x.lock().unwrap();

                *f = asyncread::ShutdownFlag::Shutdown;

                c += 1;
            }
        }

        log::debug!( "sdf_shutdown {}/{}", c, n );
    }

    pub fn make_random_token( &mut self ) -> String
    {
        let src = "0123456789abcdef".as_bytes();
        let sel : Vec< u8 > = src.choose_multiple( &mut self.rng, 16 ).cloned().collect();
        sel.iter().map( | &s | s as char ).collect::<String>()
    }

    pub fn next_notice_reply_token( &mut self ) -> String
    {
        self.notice_reply_token      = self.make_random_token();
        self.notice_reply_token_time = time::Instant::now();

        String::from( &self.notice_reply_token )
    }

    pub fn current_notice_reply_token( &self ) -> ( String, time::Duration )
    {
        ( String::from( &self.notice_reply_token ), self.notice_reply_token_time.elapsed() )
    }
}

pub fn check_urls( list : &Vec< String > ) -> Option< ConfigDynOutputError >
{
    for url in list.iter().map( | x | x.trim() ).filter( | x | *x != "" )
    {
        let ( url, _ ) = split_url_with_name( &url );

        if let Err( x ) = check_url( &url )
        {
            return Some(
                ConfigDynOutputError
                {
                    err_msg : String::from( format!( "URL ParseError {:?} [{}]", x, &url ) )
                }
            );
        };
    }

    None
}

pub fn split_url_with_name( url_with_name : &str ) -> ( String, String )
{
    lazy_static!
    {
        static ref RE : regex::Regex =
            regex::Regex::new( URL_WITH_NAME ).unwrap();
    }

    match RE.captures( &url_with_name )
    {
        Some( cap ) =>
        {
            let tail = &url_with_name[ cap.get( 0 ).unwrap().end() .. ];
            let name = cap.get( 1 ).unwrap().as_str();

            ( String::from( tail.trim() ), String::from( name.trim() ) )
        }
    ,   None =>
        {
            ( String::from( url_with_name.trim() ), String::new() )
        }
    }
}

pub fn concat_url_with_name( url : &str, name : &str ) -> String
{
    if name == ""
    {
        String::from( url.trim() )
    }
    else
    {
        format!( "[{}] {}", name.trim(), url.trim() )
    }
}

pub fn check_url( url : &str ) -> Result< (), url::ParseError >
{
    lazy_static!
    {
        static ref RE : regex::Regex =
            regex::Regex::new( IGNORE_CHECK_URL ).unwrap();
    }

    if !RE.is_match( url )
    {
        if let Err( x ) = Url::parse( &url )
        {
            return Err( x );
        }
    }

    Ok( () )
}

pub fn make_uniq_list( list : &Vec< String > ) -> Vec< String >
{
    let mut dic = HashSet::< String >::new();
    let mut ret = Vec::< String >::new();

    for t in list.iter().map( | x | String::from( x.trim() ) ).filter( | x | x != "" )
    {
        if !dic.contains( &t )
        {
            dic.insert( String::from( &t ) );
            ret.push( String::from( &t ) )
        }
    }

    ret
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
