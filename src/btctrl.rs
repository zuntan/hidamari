//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io;

use tokio::time::{ delay_for, Duration, Instant };
use tokio::sync::{ mpsc };

use serde::{ Serialize, /* Deserialize */ };

use crate::context;
use crate::event;
use crate::bt;

const DEEP_SLEEP    : Duration = Duration::from_secs( 2 );
const SHALLOW_SLEEP : Duration = Duration::from_millis( 500 );

type BtctlStatusResult<'a> = Result< &'a BtctlStatus< 'a >, () >;

#[derive(Debug, Serialize)]
pub struct BtctlStatusMember
{
    enable  : bool
,   time    : String
,   adapter : Vec< bt::BtAdapterStatus >
}

#[derive(Debug, Serialize)]
pub struct BtctlStatus<'a>
{
    bt_status : &'a BtctlStatusMember
}

#[derive(Debug, Serialize, Clone)]
pub struct BtctlOk {}

#[derive(Debug, Serialize, Clone)]
pub struct BtctlErr
{
    msg : String
}

///
pub type BtctlResult       = Result< BtctlOk, BtctlErr >;

///
#[derive(Debug)]
pub enum BtctlRequestType
{
    Nop
,   Cmd( String, String, bool )
,   Shutdown
}

///
pub struct BtctlRequest
{
    pub req  : BtctlRequestType
,   pub tx   : oneshot::Sender< BtctlResult >
}


pub async fn btctrl_task(
    arwlctx : context::ARWLContext
,   mut rx  : mpsc::Receiver< event::BtctlRequest >
)
-> io::Result< ()  >
{
    log::debug!( "btctrl start." );

    let bt_conn : Option< bt::BtConn > =
        match bt::BtConn::new().await
        {
            Ok( x ) =>
            {
                Some( x )
            }
        ,   Err( x ) =>
            {
                log::error!( "BtConn init error {:?}", x );
                None
            }
        };


    loop
    {
        // status update

        let mut btctl_st_m =
            BtctlStatusMember
            {
                enable : false
            ,   time :  chrono::Local::now().to_rfc3339()
            ,   adapter : Vec::< bt::BtAdapterStatus >::new()
            };

        match bt_conn
        {
            None =>
            {
            }
        ,   Some( ref bt_conn ) =>
            {
                btctl_st_m.enable = true;

                match bt_conn.get_adapters().await
                {
                    Ok( bt_adapters ) =>
                    {
                        for bt_adapter in bt_adapters
                        {
                            match bt_adapter.get_status( true ).await
                            {
                                Ok( bt_adapter_status ) =>
                                {
                                    btctl_st_m.adapter.push( bt_adapter_status );
                                }
                            ,   Err( x ) =>
                                {
                                    log::error!( "btctrl error {:?}", x );
                                }
                            }
                        }
                    }
                ,   Err( x ) =>
                    {
                        log::error!( "btctrl error {:?}", x );
                    }
                }
            }
        }

        // sort adapter_status

        btctl_st_m.adapter.sort_by(
            | lhs, rhs |
            {
                let lhs_name = format!( "{} [{}]", lhs.name, lhs.address );
                let rhs_name = format!( "{} [{}]", rhs.name, rhs.address );
                lhs_name.cmp( &rhs_name )
            }
        );

        // sort device_status

        for adapter_status in btctl_st_m.adapter.iter_mut()
        {
            if let Some( device_status ) = adapter_status.device_status.as_mut()
            {
                device_status.sort_by(
                    | lhs, rhs |
                    {
                        let lhs_name = format!( "{} [{}]", lhs.name, lhs.address );
                        let rhs_name = format!( "{} [{}]", rhs.name, rhs.address );
                        lhs_name.cmp( &rhs_name )
                    }
                );
            }
        }

        {
            let mut ctx = arwlctx.write().await;

            if let Ok( x ) = serde_json::to_string( &BtctlStatusResult::Ok( &BtctlStatus{ bt_status : &btctl_st_m } ) )
            {
                ctx.bt_status_json = x;
            }
        }

        match timeout( rx_time_out, rx.recv() ).await
        {
            Ok(recv) =>
            {
            }
        }

        delay_for( if btctl_st_m.enable { SHALLOW_SLEEP } else { DEEP_SLEEP } ).await;
    }

    log::debug!( "btctrl stop." );

    Ok(())
}

