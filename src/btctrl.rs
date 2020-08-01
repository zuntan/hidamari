//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io;

use std::sync::{ Arc, Mutex };

use tokio::time::{ timeout, delay_for, Duration /*, Instant */ };
use tokio::sync::{ oneshot, mpsc };

use serde::{ Serialize, /* Deserialize */ };

use async_trait::async_trait;

use crate::context;
use crate::event;
use crate::bt;

const DEEP_SLEEP    : Duration = Duration::from_millis( 3000 );
const SHALLOW_SLEEP : Duration = Duration::from_millis( 1000 );


#[derive(Debug, Serialize)]
pub struct BtctrlStatusMember
{
    enable  : bool
,   time    : String
,   adapter : Vec< bt::BtAdapterStatus >
}

type BtctrlStatusResult<'a> = Result< &'a BtctrlStatus< 'a >, () >;

#[derive(Debug, Serialize)]
pub struct BtctrlStatus<'a>
{
    bt_status : &'a BtctrlStatusMember
}

#[derive(Debug, Serialize, Clone)]
pub struct BtctrlOk {}

impl BtctrlOk
{
    fn new() -> BtctrlOk
    {
        BtctrlOk{}
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct BtctrlErr
{
    pub err_code : i32
,   pub msg_text : String
}

impl BtctrlErr
{
    fn new( err_code : i32, msg_text : &str ) -> BtctrlErr
    {
        BtctrlErr{ err_code, msg_text : String::from( msg_text ) }
    }
}

///
pub type BtctrlResult       = Result< BtctrlOk, BtctrlErr >;

///
#[derive(Debug)]
pub enum BtctrlRequestType
{
    Nop
,   Cmd( String, String, String, bool, Option< String > )
,   Reply( String, bool )
,   Shutdown
}

///
pub struct BtctrlRequest
{
    pub req  : BtctrlRequestType
,   pub tx   : oneshot::Sender< BtctrlResult >
}

impl BtctrlRequest
{
    pub fn new() -> ( BtctrlRequest, oneshot::Receiver< BtctrlResult > )
    {
        let ( tx, rx ) = oneshot::channel::< BtctrlResult >();

        (
            BtctrlRequest{
                req         : BtctrlRequestType::Nop
            ,   tx
            }
        ,   rx
        )
    }
}

type BtctrlNoticeResult<'a> = Result< &'a BtctrlNotice, () >;

#[derive(Debug, Serialize)]
pub struct BtctrlNotice
{
    title       : String
,   device      : Option< bt::BtDeviceStatus >
,   passkey     : Option< String >
,   entered     : Option< String >
,   reply_token : String
,   cancel      : bool
}

struct BtAgentIO
{
    arwlctx     : context::ARWLContext
,   reply_token : String
}

impl BtAgentIO
{
    fn new( arwlctx : context::ARWLContext ) -> BtAgentIO
    {
        BtAgentIO
        {
            arwlctx
        ,   reply_token : String::new()
        }
    }
}

#[async_trait]
impl bt::BtAgentIO for BtAgentIO
{
    // async fn request_pincode( &self, device : bt::BtDeviceStatus, pincode : &str ) -> bool { true }
    // async fn display_pincode( &self, device : bt::BtDeviceStatus, pincode : &str ) -> bool { true }

    async fn request_passkey( &self, device : bt::BtDeviceStatus, passkey : &str ) -> bool
    {
        let mut ctx = self.arwlctx.write().await;
        let token = ctx.next_bt_notice_reply_token();

        log::info!( "BtAgentIO:RequestPasskey dev {:?} ret {:06} token {}", device, passkey, token );

        let n =
            BtctrlNotice
            {
                title       : String::from( "RequestPasskey" )
            ,   device      : Some( device )
            ,   passkey     : Some( String::from( passkey ) )
            ,   entered     : None
            ,   reply_token : token
            ,   cancel      : false
            };

        if let Ok( x ) = serde_json::to_string( &BtctrlNoticeResult::Ok( &n ) )
        {
            ctx.bt_notice_json = x;
        }

        false
    }

    async fn display_passkey( &self, device : bt::BtDeviceStatus, passkey : &str, entered : &str )
    {
        let mut ctx = self.arwlctx.write().await;
        let token = ctx.next_bt_notice_reply_token();

        log::info!( "BtAgentIO:DisplayPasskey dev {:?} passkey {:?} entered {:?} token {}", device, passkey, entered, token );

        let n =
            BtctrlNotice
            {
                title       : String::from( "RequestPasskey" )
            ,   device      : Some( device )
            ,   passkey     : Some( String::from( passkey ) )
            ,   entered     : Some( String::from( entered ) )
            ,   reply_token : token
            ,   cancel      : false
            };

        if let Ok( x ) = serde_json::to_string( &BtctrlNoticeResult::Ok( &n ) )
        {
            ctx.bt_notice_json = x;
        }
    }

    async fn request_confirmation( &self, device : bt::BtDeviceStatus, passkey : &str ) -> bool
    {
        let mut ctx = self.arwlctx.write().await;
        let token = ctx.next_bt_notice_reply_token();

        log::info!( "BtAgentIO:RequestConfirmation dev {:?} ret {:06} token {}", device, passkey, token );

        let n =
            BtctrlNotice
            {
                title       : String::from( "RequestPasskey" )
            ,   device      : Some( device )
            ,   passkey     : Some( String::from( passkey ) )
            ,   entered     : None
            ,   reply_token : token
            ,   cancel      : false
            };

        if let Ok( x ) = serde_json::to_string( &BtctrlNoticeResult::Ok( &n ) )
        {
            ctx.bt_notice_json = x;
        }

        false
    }

    async fn cancel( &self )
    {
        log::debug!( "BtAgentIO:Cancel" );

        let mut ctx = self.arwlctx.write().await;
        let token = ctx.current_bt_notice_reply_token();

        log::info!( "BtAgentIO:Cancel" );

        let n =
            BtctrlNotice
            {
                title       : String::from( "Cancel" )
            ,   device      : None
            ,   passkey     : None
            ,   entered     : None
            ,   reply_token : token
            ,   cancel      : true
            };

        if let Ok( x ) = serde_json::to_string( &BtctrlNoticeResult::Ok( &n ) )
        {
            ctx.bt_notice_json = x;
        }
    }

    async fn ok( &self ) -> bool
    {
        let mut ctx = self.arwlctx.write().await;

        ctx.reset_bt_notice_reply_token();

        true
    }
}

pub async fn btctrl_task(
    arwlctx : context::ARWLContext
,   mut rx  : mpsc::Receiver< BtctrlRequest >
)
-> io::Result< ()  >
{
    log::debug!( "btctrl start." );

    let bt_conn : Option< bt::BtConn > =
        match bt::BtConn::new().await
        {
            Ok( mut bt_conn ) =>
            {
                let bt_agent_ctx    = bt::BtAgentContextImpl::new();
                let bt_agent_io     = Arc::new( BtAgentIO::new( arwlctx.clone() ) );

                bt_conn.setup_agent(
                    bt::BtAgentCapability::DisplayYesNo
                ,   bt_agent_ctx
                ,   bt_agent_io
                ).await;

                Some( bt_conn )
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
            BtctrlStatusMember
            {
                enable : false
            ,   time :  chrono::Local::now().to_rfc3339()
            ,   adapter : Vec::< bt::BtAdapterStatus >::new()
            };

        // get adapter_status

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
                let lhs_name = format!( "{}[{}]", lhs.name, lhs.address );
                let rhs_name = format!( "{}[{}]", rhs.name, rhs.address );
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
                        let lhs_name = format!( "{}{}[{}]", if lhs.paired { "1" } else { "0" }, lhs.name, lhs.address );
                        let rhs_name = format!( "{}{}[{}]", if rhs.paired { "1" } else { "0" }, rhs.name, rhs.address );
                        lhs_name.cmp( &rhs_name )
                    }
                );
            }
        }

        {
            let mut ctx = arwlctx.write().await;

            if let Ok( x ) = serde_json::to_string( &BtctrlStatusResult::Ok( &BtctrlStatus{ bt_status : &btctl_st_m } ) )
            {
                ctx.bt_status_json = x;
            }
        }

        match timeout( event::EVENT_WAIT_TIMEOUT, rx.recv() ).await
        {
            Ok( recv ) =>
            {
                let recv = recv.unwrap();

                log::debug!( "recv [{:?}]", recv.req );

                match recv.req
                {
                    BtctrlRequestType::Shutdown =>
                    {
                        recv.tx.send( Ok( BtctrlOk::new() ) ).ok();
                        break;
                    }

                ,   BtctrlRequestType::Cmd( cmd, aid, did, sw, _arg ) =>
                    {
                        if let Some( err ) =
                            match bt_conn
                            {
                                None =>
                                {
                                    Some( BtctrlErr::new( -1, "BtConn init error" ) )
                                }
                            ,   Some( ref bt_conn ) =>
                                {
                                    match bt_conn.get_adapter( &aid ).await
                                    {
                                        Ok( bt_adapter ) =>
                                        {
                                            macro_rules! exec
                                            {
                                                ( $e : expr ) =>
                                                {
                                                    if let Err( x ) = $e.await
                                                    {
                                                        Some( BtctrlErr::new( -2, &format!( "{:?}", x ) ) )
                                                    }
                                                    else { None }
                                                }
                                            }

                                            match cmd.as_str()
                                            {
                                                "ad_power" =>
                                                {
                                                    exec!( bt_adapter.set_powered( sw ) )
                                                }
                                            ,   "ad_pairable" =>
                                                {
                                                    exec!( bt_adapter.set_pairable( sw ) )
                                                }
                                            ,   "ad_discoverable" =>
                                                {
                                                    exec!( bt_adapter.set_discoverable( sw ) )
                                                }
                                            ,   "ad_discovering" =>
                                                {
                                                    if sw
                                                    {
                                                        exec!( bt_adapter.start_discovery() )
                                                    }
                                                    else
                                                    {
                                                        exec!( bt_adapter.stop_discovery() )
                                                    }
                                                }
                                            ,   "dev_remove" =>
                                                {
                                                    exec!( bt_adapter.remove_device( &did ) )
                                                }
                                            ,   "dev_connect" | "dev_pair" | "dev_trust" | "dev_block" =>
                                                {
                                                    match bt_adapter.get_device( &did ).await
                                                    {
                                                        Ok( bt_device ) =>
                                                        {
                                                            match cmd.as_str()
                                                            {
                                                                "dev_connect" =>
                                                                {
                                                                    if sw
                                                                    {
                                                                        exec!( bt_device.connect() )
                                                                    }
                                                                    else
                                                                    {
                                                                        exec!( bt_device.disconnect() )
                                                                    }
                                                                }
                                                            ,   "dev_pair" =>
                                                                {
                                                                    if sw
                                                                    {
                                                                        exec!( bt_device.pair() )
                                                                    }
                                                                    else
                                                                    {
                                                                        Some( BtctrlErr::new( -7, &format!( "Invarid parameter [{}]", &sw ) ) )
                                                                    }
                                                                }
                                                            ,   "dev_trust" =>
                                                                {
                                                                    exec!( bt_device.set_trusted( sw ) )
                                                                }
                                                            ,   "dev_block" =>
                                                                {
                                                                    exec!( bt_device.set_blocked( sw ) )
                                                                }
                                                            ,   _ => { None }
                                                            }
                                                        }
                                                    ,   Err( x ) =>
                                                        {
                                                            Some( BtctrlErr::new( -2, &format!( "{:?}", x ) ) )
                                                        }
                                                    }
                                                }
                                            ,   _ =>
                                                {
                                                    Some( BtctrlErr::new( -8, &format!( "No such command [{}]", &cmd ) ) )
                                                }
                                            }
                                        }
                                    ,   Err( x ) =>
                                        {
                                            Some( BtctrlErr::new( -2, &format!( "{:?}", x ) ) )
                                        }
                                    }
                                }
                            }
                        {
                            recv.tx.send( Err( err ) ).ok();
                        }
                        else
                        {
                            recv.tx.send( Ok( BtctrlOk::new() ) ).ok();
                        }
                    }
                ,   _ =>
                    {
                        recv.tx.send( Err( BtctrlErr::new( -9, "" ) ) ).ok();
                    }
                }
            }
        ,   Err( _ ) =>
            {
                delay_for( if btctl_st_m.enable { SHALLOW_SLEEP } else { DEEP_SLEEP } ).await;
            }
        }
    }

    log::debug!( "btctrl stop." );

    Ok(())
}

