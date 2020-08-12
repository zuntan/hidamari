//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io;

use tokio::time::{ interval, Duration, Instant };
use tokio::sync::{ mpsc };

use serde::{ Serialize /*, Deserialize */ };

use crate::context;
use crate::btctrl;
use crate::mpdcom;
use crate::mpdfifo;
use crate::event;

#[derive(Debug, Serialize)]
pub enum IoItemType
{
    AuxIn
,   MpdOut
,   BtIn
,   BtOut
}

#[derive(Debug, Serialize)]
pub struct IoItem
{
    pub r#type  : IoItemType
,   pub name    : String
,   pub url     : String
,   pub link    : Option< String >
,   pub enable  : bool
}

#[derive(Debug, Serialize)]
pub struct IoList
{
    pub io_list : Vec< IoItem >
}

type IoListResult = Result< IoList, () >;

pub async fn io_list_result( arwlctx : context::ARWLContext ) -> IoListResult
{
    IoListResult::Ok( IoList{ io_list : io_list( arwlctx ).await } )
}

pub async fn io_list( arwlctx : context::ARWLContext ) -> Vec< IoItem >
{
    let mut io_list = Vec::< IoItem >::new();

    for ( no, url ) in arwlctx.write().await.aux_in().iter().enumerate()
    {
        io_list.push(
            IoItem
            {
                r#type  : IoItemType::AuxIn
            ,   name    : format!( "AUX IN {}", no + 1 )
            ,   url     : String::from( url )
            ,   link    : None
            ,   enable  : true
            }
        );
    }

    let mpd_proxy_stream = { arwlctx.read().await.config.mpd_httpd_url != "" };

    let ( mut req, rx ) = mpdcom::MpdComRequest::new();

    req.req = mpdcom::MpdComRequestType::CmdInner( String::from( "outputs" ) );

    let _ = arwlctx.write().await.mpdcom_tx.send( req ).await;

    match rx.await
    {
        Ok(x)  =>
        {
            if let Ok( mpdcomok ) = x
            {
                let mut io_list_mpd = Vec::< IoItem >::new();

                let mut outputenabled   : Option< bool >    = None;
                let mut plugin          : Option< String >  = None;
                let mut outputname      : Option< String >  = None;

                for ( k, v ) in mpdcomok.flds.iter().rev()
                {
                    match k.as_str()
                    {
                        "outputenabled" =>
                        {
                            outputenabled = Some( v == "1" );
                        }

                    ,   "plugin"        =>
                        {
                            plugin = Some( String::from( v ) );
                        }

                    ,   "outputname"    =>
                        {
                            outputname = Some( String::from( v ) );
                        }

                    ,   "outputid"      =>
                        {
                            let outputid        = v;
                            let t_outputname    = outputname.unwrap_or( String::new() );
                            let t_plugin        = plugin.unwrap_or( String::new() );

                            let link =
                                if mpd_proxy_stream && t_plugin == "httpd"
                                {
                                    Some( String::from( context::HIDAMARI_MPD_PROXY_STREAM_PATH ) )
                                }
                                else
                                {
                                    None
                                };

                            io_list_mpd.push(
                                IoItem
                                {
                                    r#type  : IoItemType::MpdOut
                                ,   name    : t_outputname
                                ,   url     : format!( "{}{}?plugin={}", context::MPD_SINK_PROTO, outputid, t_plugin )
                                ,   link
                                ,   enable  : outputenabled.unwrap_or( false )
                                }
                            );

                            outputenabled   = None;
                            plugin          = None;
                            outputname      = None;
                        }
                    ,   _ => {}
                    }
                }

                io_list_mpd.reverse();
                io_list.append( &mut io_list_mpd );
            }
        }
    ,   Err(_) => {}
    }

    let ( mut req, rx ) = btctrl::BtctrlRequest::new();

    req.req = btctrl::BtctrlRequestType::CmdInner( String::from( "bt_status" ) );

    let _ = arwlctx.write().await.btctrl_tx.send( req ).await;

    if let Ok( x ) = rx.await
    {
        if let Ok( btctrlok ) = x
        {
            if let btctrl::BtctrlOkInner::Status( bt_status ) = btctrlok.inner
            {
                for adapter_status in bt_status.adapter
                {
                    if let Some( device_status_list ) = adapter_status.device_status
                    {
                        for device_status in device_status_list.iter()
                        {
                            if device_status.audio_source && device_status.paired
                            {
                                io_list.push(
                                    IoItem
                                    {
                                        r#type  : IoItemType::BtIn
                                    ,   name    : format!( "BT IN {} [{}]", device_status.alias, device_status.address )
                                    ,   url     : format!( "{}bluealsa:DEV={}", context::ALSA_SOURCE_PROTO, device_status.address )
                                    ,   link    : None
                                    ,   enable  : true
                                    }
                                );
                            }

                            if device_status.audio_sink && device_status.connected
                            {
                                let url = format!( "{}bluealsa:DEV={}", context::ALSA_SINK_PROTO, device_status.address );

                                let enable =
                                    {
                                        let ( mut req, rx ) = mpdfifo::MpdfifoRequest::new();

                                        req.req = mpdfifo::MpdfifoRequestType::AlsaIsEnable( String::from( &url ) );

                                        let _ = arwlctx.write().await.mpdfifo_tx.send( req ).await;

                                        if let Ok( x ) = rx.await
                                        {
                                            x.unwrap().enable.unwrap_or( false )
                                        }
                                        else
                                        {
                                            false
                                        }
                                    };

                                io_list.push(
                                    IoItem
                                    {
                                        r#type  : IoItemType::BtOut
                                    ,   name    : format!( "{} [{}]", device_status.alias, device_status.address )
                                    ,   url
                                    ,   link    : None
                                    ,   enable
                                    }
                                );
                            }
                        }
                    }
                }
            }
        }
    }



    io_list
}

const INTERVAL  : Duration = Duration::from_millis( 500 );
const EXEC_SPAN : Duration = Duration::from_millis( 1000 );

pub async fn iolist_task(
    arwlctx : context::ARWLContext
,   mut rx  : mpsc::Receiver< event::EventRequest >
)
-> io::Result< () >
{
    log::debug!( "iolist start." );

    let mut interval = interval( INTERVAL );

    interval.tick().await;

    let mut tm = Instant::now() - EXEC_SPAN;

    loop
    {
        if event::event_shutdown( &mut rx ).await
        {
            break;
        }

        if tm.elapsed() >= EXEC_SPAN
        {
            if let Ok( x ) = serde_json::to_string( &io_list_result( arwlctx.clone() ).await )
            {
                arwlctx.write().await.io_list_json = x;
            }

            tm = Instant::now();
        }

        interval.tick().await;
    }

    log::debug!( "iolist stop." );

    Ok(())
}
