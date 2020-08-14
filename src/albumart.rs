//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use std::str::FromStr;

use tokio::time::{ interval, Duration, Instant };
use tokio::sync::{ mpsc, RwLock };
use tokio::fs::File;
use tokio::prelude::*;

use hyper::{ Client, StatusCode };

use headers::HeaderMapExt;

use futures::prelude::*;

use rupnp::ssdp::{ SearchTarget, URN };
use rupnp::{ Device, Service };
use rupnp::http::uri::{ Uri, PathAndQuery };

use mime::Mime;

use lru::LruCache;

use crate::event;

const ALBUMART_CACHE_EXPIRE             : Duration = Duration::from_secs( 60 * 15 );
const ALBUMART_CACHE_MAX_ENTRY          : usize = 1024;
const _ALBUMART_BIN_CACHE_MAX_ENTRY     : usize = 256;

const UPNP_MEDIA_SERVER_DEVICE          : URN = URN::device ( "schemas-upnp-org", "MediaServer", 1 );

// http://upnp.org/specs/av/UPnP-av-ContentDirectory-v1-Service.pdf
const UPNP_CONTENT_DIRECTORY_SERVICE    : URN = URN::service( "schemas-upnp-org", "ContentDirectory", 1 );

const ALBUMART_ALT_IMG_MAXLEN           : u64 = 1024 * 256 * 1;  // 512 Kbyte

#[derive(Debug, Clone)]
pub enum AlbumartResult
{
    BadRequest
,   NotFound
,   NotFoundNoCache                             // same NotFound. use internal only
,   Binary( Mime, chrono::DateTime< chrono::Utc >, Arc< Vec< u8 > > )
}

#[derive(Debug)]
pub struct AlbumartCacheEntry
{
    pub art     : AlbumartResult
,   pub inst    : Instant
}

#[derive(Debug)]
pub struct UpnpDeviceCacheEntry
{
    pub device      : Device
,   pub inst        : Instant
}

type UpnpServiceCache = HashMap< String, ( Service, Uri ) >;

#[derive(Debug)]
pub struct AlbumartContext
{
    upnp                        : bool
,   localdir                    : String
,   cache                       : LruCache< String, AlbumartCacheEntry >
,   upnp_service_cache          : UpnpServiceCache
}

///
pub type ARWLAlbumartContext = Arc< RwLock< AlbumartContext > >;

impl AlbumartContext
{
    pub fn new( upnp : bool, localdir : &str ) -> AlbumartContext
    {
        AlbumartContext
        {
            upnp
        ,   localdir                    : String::from( localdir )
        ,   cache                       : LruCache::new( ALBUMART_CACHE_MAX_ENTRY )
        ,   upnp_service_cache          : UpnpServiceCache::new()
        }
    }
}

fn arg_browse_direct_children( objectid : &str ) -> String
{
    format!
    ( r#"
<ObjectID>{}</ObjectID>
<BrowseFlag>BrowseDirectChildren</BrowseFlag>
<Filter>*</Filter>
<StartingIndex>0</StartingIndex>
<RequestedCount>0</RequestedCount>
<SortCriteria></SortCriteria>
"#
        ,   objectid
    )
}

enum IdType
{
    Container( String )
,   Item( String, String )
}

fn find_from_xml( xml : &str, target : &str ) -> Option< IdType >
{
    match roxmltree::Document::parse( xml )
    {
        Ok( doc ) =>
        {
            if let Some( node ) = doc.descendants().find( | node | node.has_tag_name( "title" ) && node.text() == Some( target ) )
            {
                if let Some( pnode ) = node.parent()
                {
                    if pnode.has_tag_name( "container" )
                    {
                        if let Some( id ) = pnode.attribute( "id" )
                        {
                            return Some( IdType::Container( String::from( id ) ) );
                        }
                    }
                    else if pnode.has_tag_name( "item" )
                    {
                        if let Some( id ) = pnode.attribute( "id" )
                        {
                            let id = String::from( id );

                            if let Some( node ) = pnode.descendants().find( | node | node.has_tag_name( "albumArtURI" ) )
                            {
                                if let Some( album_art_uri ) = node.text()
                                {
                                    return Some( IdType::Item( id, String::from( album_art_uri ) ) );
                                }
                            }
                        }
                    }
                }
            }
        }
    ,   Err( x ) =>
        {
            log::debug!( "error:find_container_id {:?}", x )
        }
    }

    None
}

async fn get_albumart_upnp( arwlaactx : ARWLAlbumartContext, path : &str ) -> AlbumartResult
{
    let path_sp : Vec<&str> = path.split( "/" ).collect();

    if path_sp.len() < 2
    {
        log::debug!( "get_albumart_upnp:path is short" );
        return AlbumartResult::NotFound;
    }

    let device_name = path_sp[ 0 ];
    let path        = &path_sp[ 1.. ];

    log::debug!( "get_albumart_upnp:request device_name {:?} path {:?}", device_name, path );

    if let Some( ( service, url ) ) = { arwlaactx.read().await.upnp_service_cache.get( device_name ) }
    {
        let mut id : Option< IdType > = Some( IdType::Container( String::from( "0" ) ) );

        for part in path
        {
            if let Some( ref x_id ) = id
            {
                if let IdType::Container( ref xx_id ) = x_id
                {
                    let args = arg_browse_direct_children( xx_id );

                    match service.action( url, "Browse", &args ).await
                    {
                        Ok( dic ) =>
                        {
                            if let Some( xml ) = dic.get( "Result" )
                            {
                                id = find_from_xml( xml, part );
                            }
                        }
                    ,   Err( x ) =>
                        {
                            log::debug!( "error:get_albumart_upnp:action {:?}", x );
                            break;
                        }
                    }
                }
                else
                {
                    break;
                }
            }
            else
            {
                break;
            }
        }

        if let Some( id ) = id
        {
            if let IdType::Item( _, album_art_uri ) = id
            {
                match hyper::Uri::from_str( &album_art_uri )
                {
                    Ok( uri ) =>
                    {
                        let client = Client::new();

                        match client.get( uri ).await
                        {
                            Ok( res ) =>
                            {
                                if res.status() == StatusCode::OK
                                {
                                    if let Some( ctype ) = res.headers().typed_get::< headers::ContentType >()
                                    {
                                        match hyper::body::to_bytes( res ).await
                                        {
                                            Ok( bytes ) =>
                                            {
                                                return AlbumartResult::Binary( ctype.into(), chrono::Utc::now(), Arc::new( bytes.to_vec() ) )
                                            }
                                        ,   Err( x ) =>
                                            {
                                                log::warn!( "error:get_albumart_upnp:request fetch {:?} {:?}", x, album_art_uri );
                                            }
                                        }
                                    }
                                    else
                                    {
                                        log::warn!( "error:get_albumart_upnp:request fetch {:?} ContentType not found", album_art_uri );
                                    }
                                }
                                else
                                {
                                    log::warn!( "error:get_albumart_upnp:request status {:?} {:?}", res.status(), album_art_uri );
                                }
                            }
                        ,   Err( x ) =>
                            {
                                log::warn!( "error:get_albumart_upnp:request {:?} {:?}", x, album_art_uri );
                            }
                        }
                    }
                ,   Err( x ) =>
                    {
                        log::warn!( "error:get_albumart_upnp:uri {:?} {:?}", x, album_art_uri );
                    }
                }
            }
        }

        AlbumartResult::NotFound
    }
    else
    {
        AlbumartResult::NotFoundNoCache
    }
}

async fn get_albumart_localdir( _arwlaactx : ARWLAlbumartContext, path : &str, base : &str ) -> AlbumartResult
{
    let mut target = PathBuf::from( base );

    target.push( path );

    if target.is_file()
    {
        if let Some( ext ) = target.extension()
        {
            if let Some( ext ) = ext.to_str()
            {
                match ext
                {
                    "m4a" | "m4b" | "m4p" =>
                    {
                        match mp4ameta::Tag::read_from_path( &target )
                        {
                            Ok( tag ) =>
                            {
                                if let Some( artwork ) = tag.artwork()
                                {
                                    match artwork
                                    {
                                        mp4ameta::Data::Jpeg( x ) =>
                                        {
                                            log::debug!( "get_albumart_localdir mp4ameta {:?}", target );

                                            return AlbumartResult::Binary( mime::IMAGE_JPEG, chrono::Utc::now(), Arc::new( x ) )
                                        }
                                    ,   mp4ameta::Data::Png( x ) =>
                                        {
                                            log::debug!( "get_albumart_localdir mp4ameta {:?}", target );

                                            return AlbumartResult::Binary( mime::IMAGE_PNG, chrono::Utc::now(), Arc::new( x ) )
                                        }
                                    ,   _ =>
                                        {
                                        }
                                    }
                                }
                            }
                        ,   Err( x ) =>
                            {
                                log::warn!( "err:get_albumart_localdir {:?} {:?}", x, target );
                            }
                        }
                    }
                ,   "mp3" =>
                    {
                        match id3::Tag::read_from_path( &target )
                        {
                            Ok( tag ) =>
                            {
                                for pic in tag.pictures()
                                {
                                    let mime = Mime::from_str( &pic.mime_type );

                                    if let Ok( mime ) = mime
                                    {
                                        if mime == mime::IMAGE_JPEG || mime == mime::IMAGE_PNG
                                        {
                                            log::debug!( "get_albumart_localdir id3 {:?}", target );

                                            return AlbumartResult::Binary( mime, chrono::Utc::now(), Arc::new( pic.data.clone() ) )
                                        }
                                    }
                                }
                            }
                        ,   Err( x ) =>
                            {
                                log::warn!( "err:get_albumart_localdir {:?} {:?}", x, target );
                            }
                        }
                    }
                ,   "flac" =>
                    {
                        match metaflac::Tag::read_from_path( &target )
                        {
                            Ok( tag ) =>
                            {
                                for pic in tag.pictures()
                                {
                                    let mime = Mime::from_str( &pic.mime_type );

                                    if let Ok( mime ) = mime
                                    {
                                        if mime == mime::IMAGE_JPEG || mime == mime::IMAGE_PNG
                                        {
                                            log::debug!( "get_albumart_localdir metaflac {:?}", target );

                                            return AlbumartResult::Binary( mime, chrono::Utc::now(), Arc::new( pic.data.clone() ) )
                                        }
                                    }
                                }
                            }
                        ,   Err( x ) =>
                            {
                                log::warn!( "err:get_albumart_localdir {:?} {:?}", x, target );
                            }
                        }
                    }
                ,   _ =>
                    {
                        log::warn!( "not support format {:?} : get_albumart_localdir {:?}", ext, target );
                    }
                }
            }
        }
    }

    target.pop();

    let mut fnames = Vec::< String >::new();

    if let Some( x ) = target.file_name()
    {
        if let Some( x ) = x.to_str()
        {
            fnames.push( String::from( x ) );
        }
    }

    fnames.push( String::from( "cover" ) );
    fnames.push( String::from( "Folder" ) );
    fnames.push( String::from( "AlbumArtSmall" ) );

    for fname in fnames
    {
        for ext in [ "jpg", "jpeg", "png" ].iter()
        {
            let mut alt_image = PathBuf::from( base );

            alt_image.push( &format!( "{}.{}", fname, ext ) );

            if let Ok( metadata ) = alt_image.metadata()
            {
                if metadata.is_file()
                {
                    if metadata.len() < ALBUMART_ALT_IMG_MAXLEN
                    {
                        match File::open( &alt_image ).await
                        {
                            Ok( mut file ) =>
                            {
                                let mut data = Vec::<u8>::new();

                                match file.read_to_end( &mut data ).await
                                {
                                    Ok( _ ) =>
                                    {
                                        let mime = mime_guess::from_path( path ).first_or_octet_stream();

                                        log::debug!( "get_albumart_localdir alt_image {:?}", alt_image );

                                        return AlbumartResult::Binary( mime, chrono::Utc::now(), Arc::new( data ) )
                                    }
                                ,   Err( x ) =>
                                    {
                                        log::warn!( "err:get_albumart_localdir {:?} {:?}", x, alt_image );
                                    }
                                }
                            }
                        ,   Err( x ) =>
                            {
                                log::warn!( "err:get_albumart_localdir {:?} {:?}", x, alt_image );
                            }
                        }
                    }
                    else
                    {
                        log::warn!( "err:get_albumart_localdir {:?} too large {:?} lim {:?}", alt_image, metadata.len(), ALBUMART_ALT_IMG_MAXLEN );
                    }
                }
            }
        }
    }

    AlbumartResult::NotFound
}

fn check_path( path : &str ) -> Option< String >
{
    let path =
        if let Some( x ) = path.strip_prefix( "/" )
        {
            if let Some( xx ) = path.strip_suffix( "/" )
            {
                String::from( xx )
            }
            else
            {
                String::from( x )
            }
        }
        else
        {
            if let Some( xx ) = path.strip_suffix( "/" )
            {
                String::from( xx )
            }
            else
            {
                String::from( path )
            }
        };


    let mut p = Vec::< String >::new();

    for x in path.split( '/' )
    {
        match x
        {
            "\\"        => { return None; }
        ,   "" | "."    => {}
        ,   ".."        =>
            {
                if p.pop().is_none()
                {
                    return None;
                }
            }
        ,   _           => { p.push( String::from( x ) ); }
        }
    }

    Some( p.join( "/" ) )
}

const USE_CACHE : bool = true;

pub async fn get_albumart( arwlaactx : ARWLAlbumartContext, path : &str ) -> AlbumartResult
{
    if let Some( path ) = check_path( path )
    {
        // find cache
        if USE_CACHE
        {
            let mut ctx = arwlaactx.write().await;

            if let Some( x ) = ctx.cache.get( &path )
            {
                if x.inst.elapsed() < ALBUMART_CACHE_EXPIRE
                {
                    log::debug!( "get_albumart:cashe hit" );
                    return x.art.clone();
                }
            }
        }

        let ( upnp, localdir ) =
        {
            let ctx = arwlaactx.read().await;

            ( ctx.upnp, String::from( &ctx.localdir ) )
        };

        let ret =
            if ! localdir.is_empty()
            {
                log::debug!( "get_albumart:localdir" );
                get_albumart_localdir( arwlaactx.clone(), &path, &localdir ).await
            }
            else
            {
                AlbumartResult::NotFound
            };

        let ret =
            if let AlbumartResult::NotFound = ret
            {
                if upnp
                {
                    log::debug!( "get_albumart:upnp" );
                    get_albumart_upnp( arwlaactx.clone(), &path ).await
                }
                else
                {
                    ret
                }
            }
            else
            {
                ret
            };

        // cache update
        if USE_CACHE
        {
            if let AlbumartResult::NotFoundNoCache = ret
            {
                AlbumartResult::NotFound
            }
            else
            {
                let mut ctx = arwlaactx.write().await;

                ctx.cache.put( path, AlbumartCacheEntry{ art : ret.clone(), inst : Instant::now() } );

                ret
            }
        }
        else
        {
            ret
        }
    }
    else
    {
        AlbumartResult::BadRequest
    }
}

const INTERVAL                      : Duration = Duration::from_millis( 500 );
const UPNP_SERVICE_CACHE_EXPIRE     : Duration = Duration::from_secs( 60 * 15 );

pub async fn albumart_task(
    arwlaactx : ARWLAlbumartContext
,   mut rx  : mpsc::Receiver< event::EventRequest >
) -> io::Result< () >
{
    log::debug!( "albumart start." );

    let mut interval = interval( INTERVAL );

    interval.tick().await;

    let mut tm = Instant::now() - UPNP_SERVICE_CACHE_EXPIRE;

    let upnp = { arwlaactx.read().await.upnp };

    loop
    {
        if event::event_shutdown( &mut rx ).await
        {
            break;
        }

        if upnp && tm.elapsed() >= UPNP_SERVICE_CACHE_EXPIRE
        {
            log::debug!( "albumart_task:service discovering start" );

            let mut services = UpnpServiceCache::new();

            let search_target = SearchTarget::URN( UPNP_MEDIA_SERVER_DEVICE );

            match rupnp::discover( &search_target, Duration::from_secs( 3 ) ).await
            {
                Ok( devices ) =>
                {
                    futures::pin_mut!( devices );

                    loop
                    {
                        match devices.try_next().await
                        {
                            Ok( device ) =>
                            {
                                if let Some( device ) = device
                                {
                                    if let Some( service ) = device.find_service( &UPNP_CONTENT_DIRECTORY_SERVICE )
                                    {
                                        services.insert(
                                            String::from( device.friendly_name() )
                                        ,   ( service.clone(), device.url().clone() )
                                        );
                                    }
                                }
                                else
                                {
                                    break;
                                }
                            }
                        ,   Err( x ) =>
                            {
                                log::debug!( "error:get_albumart_upnp:try_next {:?}", x )
                            }
                        }
                    }
                }
            ,   Err( x ) =>
                {
                    log::error!( "error:get_albumart_upnp:discover {:?}", x )
                }
            }

            log::debug!( "albumart_task:service discovering end {:?}", services );

            if false /* log::log_enabled!( log::Level::Debug ) */
            {
                lazy_static!
                {
                    static ref RE : regex::Regex =
                        regex::Regex::new( "control_endpoint: ([^,]+)," ).unwrap();
                }

                for ( k, v ) in services.iter()
                {
                    let ep =
                        match RE.captures( &format!( "{:?}", v.0 ) )
                        {
                            Some( cap ) =>
                            {
                                String::from( cap.get( 1 ).unwrap().as_str() )
                            }
                        ,   _ =>
                            {
                                String::new()
                            }
                        };

                    let ep =
                        match PathAndQuery::from_str( &ep )
                        {
                            Ok( x )     => { x }
                        ,   Err( _ )    => { PathAndQuery::from_static( "/" ) }
                        };

                    let mut parts = v.1.clone().into_parts();
                    parts.path_and_query = Some( ep );

                    let uri = Uri::from_parts( parts );

                    log::debug!( "Found upnp media server. [{:?}] {:?}", k, uri );
                }
            }

            if log::log_enabled!( log::Level::Info )
            {
                let mut a : Vec< String > =
                    arwlaactx.read().await.upnp_service_cache.keys().map( | x | String::from( x ) ).collect();

                let mut b : Vec< String > =
                    services.keys().map( | x | String::from( x ) ).collect();

                a.sort();
                b.sort();

                if a != b
                {
                    log::info!( "Update upnp media server. {:?}", b );
                }
            }

            let mut ctx = arwlaactx.write().await;

            ctx.upnp_service_cache = services;

            tm = Instant::now();
        }

        interval.tick().await;
    }

    log::debug!( "albumart stop." );

    Ok(())
}
