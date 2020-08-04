//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::sync::{ Arc, Mutex };
use std::collections::HashMap;
use std::pin::Pin;
use std::ops::Fn;

use rand::prelude::*;

use tokio::time::{ Duration };

use serde::{ Serialize, /* Deserialize */ };

use async_trait::async_trait;

use dbus_tokio::connection;

use dbus::nonblock::{ SyncConnection, Proxy, stdintf::org_freedesktop_dbus::Properties /*, MethodReply, MsgMatch */ };
use dbus::message::{ MatchRule /*, MessageType */ };
use dbus::strings::{ Path /*, Interface, BusName, Member */ };
use dbus::arg::{ Variant, RefArg };
use dbus::channel::MatchingReceiver;

use dbus_crossroads::{ MethodErr, Crossroads, IfaceBuilder /*, IfaceToken */ };

/*
static BLUEZ_SENDER                 : &'static str = "org.bluez";
*/
static BLUEZ_SERVICE_NAME           : &'static str = "org.bluez";

static OBJECT_MANAGER_INTERFACE     : &'static str = "org.freedesktop.DBus.ObjectManager";
static GET_MANAGED_OBJECTS          : &'static str = "GetManagedObjects";

static BLUEZ_ADAPTER_INTERFACE      : &'static str = "org.bluez.Adapter1";
static BLUEZ_DEVICE_INTERFACE       : &'static str = "org.bluez.Device1";
static BLUEZ_AGENT_INTERFACE        : &'static str = "org.bluez.Agent1";

static BLUEZ_AGENT_MANAGER_PATH     : &'static str = "/org/bluez";
static BLUEZ_AGENT_MANAGER_INTERFACE : &'static str = "org.bluez.AgentManager1";

static BLUEZ_AGENT_PATH             : &'static str = "/net/zuntan/bt";

static BLUEZ_ERROR_REJECTED         : &'static str = "org.bluez.Error.Rejected";
static BLUEZ_ERROR_CANCELED         : &'static str = "org.bluez.Error.Canceled";

static REQUEST_PINCODE              : &'static str = "0000";
static REQUEST_PASSKEY              : u32 = 0;

pub static AUDIO_SOURCE_UUID        : &'static str = "0000110a-0000-1000-8000-00805f9b34fb";
pub static AUDIO_SINK_UUID          : &'static str = "0000110b-0000-1000-8000-00805f9b34fb";

const TIME_OUT                      : Duration = Duration::from_secs( 3 );

pub type Result< T >    = std::result::Result< T, dbus::Error >;

type MethodResult< T >  = std::result::Result< T, MethodErr >;

pub type GetManagedObjectsRetType<'a> =
    HashMap< dbus::strings::Path<'a>, HashMap< String, HashMap< String, Variant< Box< dyn RefArg > > > > >;

pub async fn get_managed_objects<'a>( conn : Arc< SyncConnection > ) -> Result< GetManagedObjectsRetType<'a> >
{
    let proxy = Proxy::new( BLUEZ_SERVICE_NAME, "/", TIME_OUT, conn );

    match proxy.method_call::< ( GetManagedObjectsRetType, ), _, _, _ >( OBJECT_MANAGER_INTERFACE, GET_MANAGED_OBJECTS, () ).await
    {
        Ok( x ) =>
        {
            Ok( x.0 )
        }
    ,   Err( x ) =>
        {
            log::debug!( "{:?}", x );
            Err( x )
        }
    }
}

pub fn pretty_dump_managed_objects<'a>( mo : &GetManagedObjectsRetType<'a> ) -> String
{
    let mut keys_1 : Vec< dbus::strings::Path<'a> > = Vec::new();

    for x in mo.keys()
    {
        keys_1.push( x.clone() );
    }

    keys_1.sort();

    // log::debug!( "{:?}", keys_1 );

    let mut sink = String::new();

    sink += "\n{\n";

    for ( i1, k1 ) in keys_1.iter().enumerate()
    {
        sink += &format!( "{}\t{:?} :", if i1 == 0 { "" } else { "," }, &k1 );

        let dic2 = &mo.get( &k1 ).unwrap();

        if dic2.is_empty()
        {
            sink += " {}\n";
        }
        else
        {
            sink += "\n\t{\n";

            let mut keys_2 = dic2.keys().collect::< Vec< &String > >();

            keys_2.sort();

            for ( i2, k2 ) in keys_2.iter().enumerate()
            {
                sink += &format!( "\t{}\t{:?}", if i2 == 0 { "" } else { "," }, &k2 );

                let dic3 = &dic2.get( *k2 ).unwrap();

                if dic3.is_empty()
                {
                    sink += " : {}\n";
                }
                else
                {
                    sink += "\n\t\t{\n";

                    let mut keys_3 = dic3.keys().collect::< Vec< &String > >();

                    keys_3.sort();

                    for ( i3, k3 ) in keys_3.iter().enumerate()
                    {

                        let val = &dic3.get( *k3 ).unwrap();


                        sink += &format!( "\t\t{}\t{:?} : {:?}\n", if i3 == 0 { "" } else { "," }, &k3, &val );
                    }

                    sink += "\t\t}\n";
                }
            }

            sink += "\t}\n";
        }
    }

    sink += "}\n";

    sink
}

pub async fn get_adapter_paths( conn : Arc< SyncConnection > ) -> Result< Vec< String > >
{
    let mut ret = Vec::<String>::new();

    let mo = get_managed_objects( conn ).await?;

    for ( k, v ) in mo.iter()
    {
        if v.contains_key( BLUEZ_ADAPTER_INTERFACE )
        {
            ret.push( k.to_string() );
        }
    }

    Ok( ret )
}

pub async fn get_device_path( conn : Arc< SyncConnection >, adapter_path : &str ) -> Result< Vec< String > >
{
    let mut ret = Vec::<String>::new();

    let mo = get_managed_objects( conn ).await?;

    for ( k, v ) in mo.iter()
    {
        if v.contains_key( BLUEZ_DEVICE_INTERFACE )
        {
            let prop = v.get( BLUEZ_DEVICE_INTERFACE ).unwrap();

            if let Some( x ) = prop.get( "Adapter" )
            {
                let adapter_path_ref = x.0.as_str().unwrap();

                if adapter_path_ref == adapter_path
                {
                    ret.push( k.to_string() );
                }
            }
        }
    }

    Ok( ret )
}

pub async fn get_adapter_path_from_device_path( conn : Arc< SyncConnection >, device_path : &str ) -> Result< String >
{
    let mo = get_managed_objects( conn ).await?;

    if let Some( v ) = mo.get( &dbus::strings::Path::from( device_path ) )
    {
        if v.contains_key( BLUEZ_DEVICE_INTERFACE )
        {
            let prop = v.get( BLUEZ_DEVICE_INTERFACE ).unwrap();

            if let Some( x ) = prop.get( "Adapter" )
            {
                let adapter_path_ref = x.0.as_str().unwrap();

                return Ok( String::from( adapter_path_ref ) )
            }
        }
    }

    Err( dbus::Error::new_custom( "Error", "Bt adapter not found" ) )
}

pub async fn call_void_func( conn : Arc< SyncConnection >, path : &str, interface : &str, func_name : &str ) -> Result< () >
{
    let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, conn );

    match proxy.method_call::< (), _, _, _ >( interface, func_name, () ).await
    {
        Ok( _ ) =>
        {
            Ok( () )
        }
    ,   Err( x ) =>
        {
            log::debug!( "{:?}", x );
            Err( x )
        }
    }
}

pub async fn call_void_func_a< T : dbus::arg::Arg + dbus::arg::Append >
    ( conn : Arc< SyncConnection >, path : &str, interface : &str, func_name : &str, value : T )
    -> Result< () >
{
    let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, conn );

    match proxy.method_call::< (), _, _, _ >( interface, func_name, ( value, ) ).await
    {
        Ok( _ ) =>
        {
            Ok( () )
        }
    ,   Err( x ) =>
        {
            log::debug!( "{:?}", x );
            Err( x )
        }
    }
}

pub async fn set< T : dbus::arg::Arg + dbus::arg::Append >
    ( conn : Arc< SyncConnection >, path : &str, interface : &str, key : &str, value : T )
    -> Result< () >
{
    let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, conn );

    match proxy.set( interface, key, value ).await
    {
        Ok( _ ) =>
        {
            Ok( () )
        }
    ,   Err( x ) =>
        {
            log::debug!( "{:?}", x );
            Err( x )
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BtAdapterStatus
{
    pub id                  : String
,   pub address             : String
,   pub address_type        : String
,   pub alias               : String
,   pub class               : u32
,   pub discoverable        : bool
,   pub discoverable_timeout : u32
,   pub discovering         : bool
,   pub modalias            : Option< String >
,   pub name                : String
,   pub pairable            : bool
,   pub pairable_timeout    : u32
,   pub powered             : bool
,   pub uuids               : Vec< String >

,   pub device_status       : Option< Vec< BtDeviceStatus > >
}

#[derive(Debug, Serialize)]
pub struct BtDeviceStatus
{
    pub id              : String
,   pub adapter         : String
,   pub address         : String
,   pub address_type    : String
,   pub alias           : String
,   pub appearance      : Option< i16 >
,   pub blocked         : bool
,   pub class           : Option< u32 >
,   pub connected       : bool
,   pub icon            : Option< String >
,   pub legacy_pairing  : bool
,   pub modalias        : Option< String >
,   pub name            : Option< String >
,   pub paired          : bool
,   pub rssi            : Option< i16 >
,   pub services_resolved : bool
,   pub trusted         : bool
,   pub tx_power        : Option< i16 >
,   pub uuids           : Option< Vec< String > >
,   pub wake_allowed    : Option< bool >
,   pub audio_source    : bool
,   pub audio_sink      : bool
}

pub async fn get_device_status( conn : Arc< SyncConnection >, path : &str ) -> Result< BtDeviceStatus >
{
    let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, conn );

    match proxy.get_all( BLUEZ_DEVICE_INTERFACE ).await
    {
        Ok( props ) =>
        {
            let adapter         = String::from( props.get( "Adapter" ).unwrap().0.as_str().unwrap() );
            let address         = String::from( props.get( "Address" ).unwrap().0.as_str().unwrap() );
            let address_type    = String::from( props.get( "AddressType" ).unwrap().0.as_str().unwrap() );
            let alias           = String::from( props.get( "Alias" ).unwrap().0.as_str().unwrap() );

            let appearance =
                if props.contains_key( "Appearance" )
                {
                    Some( *props.get( "Appearance" ).unwrap().0.as_any().downcast_ref::<i16>().unwrap() )
                }
                else
                {
                    None
                };

            let blocked         = *props.get( "Blocked" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();

            let class           =
                if props.contains_key( "Class" )
                {
                    Some( *props.get( "Class" ).unwrap().0.as_any().downcast_ref::<u32>().unwrap() )
                }
                else
                {
                    None
                };

            let connected       = *props.get( "Connected" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();

            let icon            =
                if props.contains_key( "Icon" )
                {
                    Some( String::from( props.get( "Icon" ).unwrap().0.as_str().unwrap() ) )
                }
                else
                {
                    None
                };

            let legacy_pairing  = *props.get( "LegacyPairing" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();

            let modalias =
                if props.contains_key( "Modalias" )
                {
                    Some( String::from( props.get( "Modalias" ).unwrap().0.as_str().unwrap() ) )
                }
                else
                {
                    None
                };

            let name            =
                if props.contains_key( "Name" )
                {
                    Some( String::from( props.get( "Name" ).unwrap().0.as_str().unwrap() ) )
                }
                else
                {
                    None
                };

            let paired          = *props.get( "Paired" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();

            let rssi =
                if props.contains_key( "RSSI" )
                {
                    Some( *props.get( "RSSI" ).unwrap().0.as_any().downcast_ref::<i16>().unwrap() )
                }
                else
                {
                    None
                };

            let services_resolved = *props.get( "ServicesResolved" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();
            let trusted         = *props.get( "Trusted" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();

            let tx_power =
                if props.contains_key( "TxPower" )
                {
                    Some( *props.get( "TxPower" ).unwrap().0.as_any().downcast_ref::<i16>().unwrap() )
                }
                else
                {
                    None
                };

            let uuids : Option< Vec< String > > =
                if props.contains_key( "UUIDs" )
                {
                    Some( props.get( "UUIDs" ).unwrap().0.as_any().downcast_ref::< Vec< String > >().unwrap().iter().map( | x | String::from( x ) ).collect() )
                }
                else
                {
                    None
                };

            let wake_allowed    =
                if props.contains_key( "WakeAllowed" )
                {
                    Some( *props.get( "WakeAllowed " ).unwrap().0.as_any().downcast_ref::<bool>().unwrap() )
                }
                else
                {
                    None
                };

            let audio_source    = if uuids.is_none() { false } else { uuids.as_ref().unwrap().iter().find( | &x | x == AUDIO_SOURCE_UUID ).is_some() };
            let audio_sink      = if uuids.is_none() { false } else { uuids.as_ref().unwrap().iter().find( | &x | x == AUDIO_SINK_UUID ).is_some() };

            Ok(
                BtDeviceStatus
                {
                    id              : String::from( path )
                ,   adapter
                ,   address
                ,   address_type
                ,   alias
                ,   appearance
                ,   blocked
                ,   class
                ,   connected
                ,   icon
                ,   legacy_pairing
                ,   modalias
                ,   name
                ,   paired
                ,   rssi
                ,   services_resolved
                ,   trusted
                ,   tx_power
                ,   uuids
                ,   wake_allowed
                ,   audio_source
                ,   audio_sink
                }
            )
        }
    ,   Err( x ) =>
        {
            log::debug!( "{:?}", x );
            Err( x )
        }
    }
}

pub async fn get_adapter_status( conn : Arc< SyncConnection >, path : &str, with_devices : bool ) -> Result< BtAdapterStatus >
{
    let device_status : Option< Vec< BtDeviceStatus > > =
        if with_devices
        {
            let mut device_status = Vec::< BtDeviceStatus >::new();

            match get_device_path( conn.clone(), path ).await
            {
                Ok( devices ) =>
                {
                    for device in devices
                    {
                        match get_device_status( conn.clone(), &device ).await
                        {
                            Ok( x ) =>
                            {
                                device_status.push( x );
                            }
                        ,   Err( x ) =>
                            {
                                log::debug!( "{:?}", x );
                                return Err( x );
                            }
                        }
                    }
                }
            ,   Err( x ) =>
                {
                    log::debug!( "{:?}", x );
                    return Err( x );
                }
            }

            Some( device_status )
        }
        else
        {
            None
        };

    let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, conn.clone() );

    match proxy.get_all( BLUEZ_ADAPTER_INTERFACE ).await
    {
        Ok( props ) =>
        {
            let address         = String::from( props.get( "Address" ).unwrap().0.as_str().unwrap() );
            let address_type    = String::from( props.get( "AddressType" ).unwrap().0.as_str().unwrap() );
            let alias           = String::from( props.get( "Alias" ).unwrap().0.as_str().unwrap() );
            let class           = *props.get( "Class" ).unwrap().0.as_any().downcast_ref::<u32>().unwrap();
            let discoverable    = *props.get( "Discoverable" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();
            let discoverable_timeout = *props.get( "DiscoverableTimeout" ).unwrap().0.as_any().downcast_ref::<u32>().unwrap();
            let discovering     = *props.get( "Discovering" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();

            let modalias        =
                if props.contains_key( "Modalias" )
                {
                    Some( String::from( props.get( "Modalias" ).unwrap().0.as_str().unwrap() ) )
                }
                else
                {
                    None
                };

            let name            = String::from( props.get( "Name" ).unwrap().0.as_str().unwrap() );
            let pairable        = *props.get( "Pairable" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();
            let pairable_timeout = *props.get( "PairableTimeout" ).unwrap().0.as_any().downcast_ref::<u32>().unwrap();
            let powered         = *props.get( "Powered" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();

            let uuids : Vec< String > = props.get( "UUIDs" ).unwrap().0.as_any().downcast_ref::< Vec<String> >().unwrap().iter().map( | x | String::from( x ) ).collect();

            Ok(
                BtAdapterStatus
                {
                    id          : String::from( path )
                ,   address
                ,   address_type
                ,   alias
                ,   class
                ,   discoverable
                ,   discoverable_timeout
                ,   discovering
                ,   modalias
                ,   name
                ,   pairable
                ,   pairable_timeout
                ,   powered
                ,   uuids
                ,   device_status
                }
            )
        }
    ,   Err( x ) =>
        {
            log::debug!( "{:?}", x );
            Err( x )
        }
    }
}

#[derive(Debug)]
pub enum BtAgentCapability
{
    DisplayOnly
,   DisplayYesNo
,   KeyboardOnly
,   NoInputNoOutput
,   KeyboardDisplay
}

impl From< BtAgentCapability > for String
{
    fn from( x: BtAgentCapability ) -> String
    {
        String::from(
            match x
            {
                BtAgentCapability::DisplayOnly      => "DisplayOnly"
            ,   BtAgentCapability::DisplayYesNo     => "DisplayYesNo"
            ,   BtAgentCapability::KeyboardOnly     => "KeyboardOnly"
            ,   BtAgentCapability::NoInputNoOutput  => "NoInputNoOutput"
            ,   BtAgentCapability::KeyboardDisplay  => "KeyboardDisplay"
            }
        )
    }
}

pub trait BtAgentContext
{
    fn make_pincode( &mut self ) -> String
    {
        String::from( REQUEST_PINCODE )
    }

    fn make_passkey( &mut self ) -> u32
    {
        REQUEST_PASSKEY
    }
}

pub struct BtAgentContextImpl
{
    rng : StdRng
}

impl BtAgentContextImpl
{
    pub fn new() -> BtAgentContextImpl
    {
        BtAgentContextImpl { rng : SeedableRng::from_rng( thread_rng() ).unwrap() }
    }
}

impl BtAgentContext for BtAgentContextImpl
{
    fn make_pincode( &mut self ) -> String
    {
        // https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/agent-api.txt
        //  RequestPinCode
        //  The return value should be a string of 1-16 characters
        //  length. The string can be alphanumeric.

        let src = "0123456789".as_bytes();
        let sel : Vec< u8 > = src.choose_multiple( &mut self.rng, 4 ).cloned().collect();

        sel.iter().map( | &s | s as char ).collect::<String>()
    }

    fn make_passkey( &mut self ) -> u32
    {
        // https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/agent-api.txt
        //  RequestPasskey
        //  The return value should be a numeric value
        //  between 0-999999.

        self.rng.gen_range( 0, 1000000 )
   }
}

pub enum BtAgentIOConfirm
{
    Reject
,   Accept
,   Confirm
}

#[async_trait]
pub trait BtAgentIO
{
    ///  BtAgentIOConfirm::Confirm returned after call fn ok()
    async fn request_pincode( &self, device : BtDeviceStatus, pincode : &str ) -> BtAgentIOConfirm
    {
        log::debug!( "BtAgentIO:RequestPinCode dev {:?} ret {:?}", device, pincode );
        BtAgentIOConfirm::Accept
    }

    ///  BtAgentIOConfirm::Confirm returned after call fn ok()
    async fn display_pincode( &self, device : BtDeviceStatus, pincode : &str ) -> BtAgentIOConfirm
    {
        log::debug!( "BtAgentIO:DisplayPinCode dev {:?} pincode {}", device, pincode );
        BtAgentIOConfirm::Accept
    }

    ///  BtAgentIOConfirm::Confirm returned after call fn ok()
    async fn request_passkey( &self, device : BtDeviceStatus, passkey : &str ) -> BtAgentIOConfirm
    {
        log::debug!( "BtAgentIO:RequestPasskey dev {:?} ret {:06}", device, passkey );
        BtAgentIOConfirm::Accept
    }

    async fn display_passkey( &self, device : BtDeviceStatus, passkey : &str, entered : &str )
    {
        log::debug!( "BtAgentIO:DisplayPasskey dev {:?} passkey {:?} entered {:?}", device, passkey, entered );
    }

    ///  BtAgentIOConfirm::Confirm returned after call fn ok()
    async fn request_confirmation( &self, device : BtDeviceStatus, passkey : &str ) -> BtAgentIOConfirm
    {
        log::debug!( "BtAgentIO:RequestConfirmation dev {:?} ret {:06}", device, passkey );
        BtAgentIOConfirm::Accept
    }

    ///  BtAgentIOConfirm::Confirm returned after call fn ok()
    async fn request_authorization( &self, device : BtDeviceStatus ) -> BtAgentIOConfirm
    {
        log::debug!( "BtAgentIO:RequestAuthorization dev {:?}", device );
        BtAgentIOConfirm::Accept
    }

    ///  BtAgentIOConfirm::Confirm returned after call fn ok()
    async fn authorize_service( &self, device : BtDeviceStatus, uuid : &str ) -> BtAgentIOConfirm
    {
        log::debug!( "BtAgentIO:AuthorizeService dev {:?} uuid {:?}", device, uuid );
        BtAgentIOConfirm::Accept
    }

    async fn cancel( &self )
    {
        log::debug!( "BtAgentIO:Cancel" );
    }

    async fn ok( &self ) -> bool
    {
        true
    }
}

pub struct BtConn
{
    conn        : Arc< SyncConnection >
,   res_err     : Arc< Mutex< Option< String > > >
,   agent       : bool
}

pub struct BtAdapter
{
    conn        : Arc< SyncConnection >
,   path        : String
}

pub struct BtDevice
{
    conn        : Arc< SyncConnection >
,   path        : String
}

impl BtConn
{
    pub async fn new() -> Result< BtConn >
    {
        let ( resource, conn ) = connection::new_system_sync()?;

        let res_err = Arc::new( Mutex::new( Option::< String >::None ) );

        let wake_res_err = Arc::downgrade( &res_err );

        tokio::spawn(
            async move
            {
                let err = resource.await;

                if let Some( res_err ) = wake_res_err.upgrade()
                {
                    let mut sink = res_err.lock().unwrap();

                    let err_str = format!( "{:?}", err );

                    log::error!( "dbus resource error. [{}]", &err_str );

                    *sink = Some( err_str );
                }
            }
        );

        /*
        let mut mr = MatchRule::new();

        mr.msg_type = Some( MessageType::Signal );
        mr.sender = Some( BLUEZ_SENDER.into() );
        mr.interface = Some( OBJECT_MANAGER_INTERFACE.into() );

        match conn.add_match( mr ).await
        {
            Ok( x ) =>
            {
                x.cb(
                    | _, ( path, intf ) : ( Path, Interface ) |
                    {
                        log::debug!( "{:?} {:?}", path, intf );
                        true
                    }
                );
            }
        ,   Err( x ) =>
            {
                log::debug!( "{:?}", x );
            }
        }
        */

        Ok(
            BtConn
            {
                conn
            ,   res_err
            ,   agent : false
            }
        )
    }

    pub async fn get_adapter( &self, path : &str ) -> Result< BtAdapter >
    {
        let paths = get_adapter_paths( self.conn.clone() ).await?;

        if let Some( _ ) = paths.iter().find( |x| *x == path )
        {
            Ok( BtAdapter{ conn : self.conn.clone(), path : String::from( path ) } )
        }
        else
        {
            Err( dbus::Error::new_custom( "Error", "Bt adapter not found" ) )
        }
    }

    pub fn get_adapter_uncheck( &self, path : &str ) -> BtAdapter
    {
        BtAdapter{ conn : self.conn.clone(), path : String::from( path ) }
    }

    pub async fn get_first_adapter( &self ) -> Result< BtAdapter >
    {
        let paths = get_adapter_paths( self.conn.clone() ).await?;

        if !paths.is_empty()
        {
            Ok( BtAdapter{ conn : self.conn.clone(), path : String::from( paths.first().unwrap() ) } )
        }
        else
        {
            Err( dbus::Error::new_custom( "Error", "Bt adapter not found" ) )
        }
    }

    pub async fn get_adapters( &self ) -> Result< Vec< BtAdapter > >
    {
        let paths = get_adapter_paths( self.conn.clone() ).await?;
        Ok( paths.iter().map( | x | BtAdapter { conn : self.conn.clone(), path : String::from( x ) } ).collect() )
    }

    pub async fn setup_agent< T : BtAgentContext + Send + 'static, U : BtAgentIO + Sync + Send + 'static >(
            &mut self
        ,   capability      : BtAgentCapability
        ,   agent_ctx       : T
        ,   agent_io        : Arc< U >
        )
    {
        if self.agent
        {
            log::error!( "error. setup_agent already done." );
            return
        }

        let mut cr = Crossroads::new();

        cr.set_async_support(
            Some(
                (
                    self.conn.clone()
                ,   Box::new( |x| { tokio::spawn( x ); } )
                )
            )
        );

        let iface_token =
            cr.register
            (
                BLUEZ_AGENT_INTERFACE
            ,   | b: &mut IfaceBuilder< T > |
                {
                    b.method(
                        "Release", (), ()
                    ,   | _, _btactx, _ : () |
                        {
                            log::debug!( "m:Release" );
                            Ok(())
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "RequestPinCode", ( "device", ), ( "pincode",  )
                    ,   move | mut ctx, cr, ( device, ) : ( Path, ) |
                        {
                            let btactx: &mut T = cr.data_mut( ctx.path() ).unwrap();

                            let pincpde = btactx.make_pincode();

                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                ctx.reply(
                                    if let Ok( device_status ) = get_device_status( conn_clone, &device ).await
                                    {
                                        match agent_io_clone.request_pincode( device_status, &pincpde ).await
                                        {
                                            BtAgentIOConfirm::Reject =>
                                            {
                                                MethodResult::<( String, )>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                            }
                                        ,   BtAgentIOConfirm::Accept =>
                                            {
                                                Ok( ( pincpde, ) )
                                            }
                                        ,   BtAgentIOConfirm::Confirm =>
                                            {
                                                if agent_io_clone.ok().await
                                                {
                                                    Ok( ( pincpde, ) )
                                                }
                                                else
                                                {
                                                    MethodResult::<( String, )>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                }
                                            }
                                        }
                                    }
                                    else
                                    {
                                        MethodResult::<( String, )>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                    }
                                )
                            }
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "DisplayPinCode", ( "device", "pincode", ), ()
                    ,   move | mut ctx, _cr, ( device, pincode, ) : ( Path, String, ) |
                        {
                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                ctx.reply(
                                    match get_device_status( conn_clone, &device ).await
                                    {
                                        Ok( device_status ) =>
                                        {
                                            match agent_io_clone.display_pincode( device_status, &pincode ).await
                                            {
                                                BtAgentIOConfirm::Reject =>
                                                {
                                                    MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                }
                                            ,   BtAgentIOConfirm::Accept =>
                                                {
                                                    Ok( () )
                                                }
                                            ,   BtAgentIOConfirm::Confirm =>
                                                {
                                                    if agent_io_clone.ok().await
                                                    {
                                                        Ok( () )
                                                    }
                                                    else
                                                    {
                                                        MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                    }
                                                }
                                            }
                                        }
                                    ,   Err( _ ) =>
                                        {
                                            MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                        }
                                    }
                                )
                            }
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "RequestPasskey", ( "device", ), ( "passkey", )
                    ,   move | mut ctx, cr, ( device, ) : ( Path, ) |
                        {
                            let btactx: &mut T = cr.data_mut( ctx.path() ).unwrap();

                            let passkey = btactx.make_passkey();

                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                ctx.reply(
                                    if let Ok( device_status ) = get_device_status( conn_clone, &device ).await
                                    {
                                        match agent_io_clone.request_passkey( device_status, &format!( "{:06}", passkey ) ).await
                                        {
                                            BtAgentIOConfirm::Reject =>
                                            {
                                                MethodResult::<( u32, )>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                            }
                                        ,   BtAgentIOConfirm::Accept =>
                                            {
                                                Ok( ( passkey, ) )
                                            }
                                        ,   BtAgentIOConfirm::Confirm =>
                                            {
                                                if agent_io_clone.ok().await
                                                {
                                                     Ok( ( passkey, ) )
                                                }
                                                else
                                                {
                                                    MethodResult::<( u32, )>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                }
                                            }
                                        }
                                    }
                                    else
                                    {
                                        MethodResult::<( u32, )>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                    }
                                )
                            }
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "DisplayPasskey", ( "device", "passkey", "entered" ), ()
                    ,   move | mut ctx, _cr, ( device, passkey, entered ) : ( Path, u32, u16 ) |
                        {
                            let passkey = format!( "{:06}", passkey );
                            let entered = format!( "{:06}", entered );

                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                ctx.reply(
                                    match get_device_status( conn_clone, &device ).await
                                    {
                                        Ok( device_status ) =>
                                        {
                                            agent_io_clone.display_passkey( device_status, &passkey, &entered ).await;

                                            Ok( () )
                                        }
                                    ,   Err( _ ) =>
                                        {
                                            MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                        }
                                    }
                                )
                            }
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "RequestConfirmation", ( "device", "passkey" ), ()
                    ,   move | mut ctx, _cr, ( device, passkey ) : ( Path, u32 ) |
                        {
                            let passkey = format!( "{:06}", passkey );

                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                ctx.reply(
                                    match get_device_status( conn_clone, &device ).await
                                    {
                                        Ok( device_status ) =>
                                        {
                                            match agent_io_clone.request_confirmation( device_status, &passkey ).await
                                            {
                                                BtAgentIOConfirm::Reject =>
                                                {
                                                    MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                }
                                            ,   BtAgentIOConfirm::Accept =>
                                                {
                                                    Ok( () )
                                                }
                                            ,   BtAgentIOConfirm::Confirm =>
                                                {
                                                    if agent_io_clone.ok().await
                                                    {
                                                         Ok( () )
                                                    }
                                                    else
                                                    {
                                                        MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                    }
                                                }
                                            }
                                        }
                                    ,   Err( _ ) =>
                                        {
                                            MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                        }
                                    }
                                )
                            }
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "RequestAuthorization", ( "device", ), ()
                    ,   move | mut ctx, _cr, ( device, ) : ( Path, ) |
                        {
                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                ctx.reply(
                                    match get_device_status( conn_clone, &device ).await
                                    {
                                        Ok( device_status ) =>
                                        {
                                            match agent_io_clone.request_authorization( device_status ).await
                                            {
                                                BtAgentIOConfirm::Reject =>
                                                {
                                                    MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                }
                                            ,   BtAgentIOConfirm::Accept =>
                                                {
                                                     Ok( () )
                                                }
                                            ,   BtAgentIOConfirm::Confirm =>
                                                {
                                                    if agent_io_clone.ok().await
                                                    {
                                                         Ok( () )
                                                    }
                                                    else
                                                    {
                                                        MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                    }
                                                }
                                            }
                                        }
                                    ,   Err( _ ) =>
                                        {
                                            MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                        }
                                    }
                                )
                            }
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "AuthorizeService", ( "device", "uuid" ), ()
                    ,   move | mut ctx, _cr, ( device, uuid ) : ( Path, String ) |
                        {
                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                ctx.reply(
                                    match get_device_status( conn_clone, &device ).await
                                    {
                                        Ok( device_status ) =>
                                        {
                                            match agent_io_clone.authorize_service( device_status, &uuid ).await
                                            {
                                                BtAgentIOConfirm::Reject =>
                                                {
                                                    MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                }
                                            ,   BtAgentIOConfirm::Accept =>
                                                {
                                                    Ok( () )
                                                }
                                            ,   BtAgentIOConfirm::Confirm =>
                                                {
                                                    if agent_io_clone.ok().await
                                                    {
                                                         Ok( () )
                                                    }
                                                    else
                                                    {
                                                        MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                                    }
                                                }
                                            }
                                        }
                                    ,   Err( _ ) =>
                                        {
                                            MethodResult::<()>::Err( MethodErr::from( ( BLUEZ_ERROR_REJECTED, "" ) ) )
                                        }
                                    }
                                )
                            }
                        }
                    );

                    let conn_clone = self.conn.clone();
                    let agent_io_clone = agent_io.clone();

                    b.method_with_cr_async(
                        "Cancel", (), ()
                    ,   move | mut ctx, _cr, _ : () |
                        {
                            let conn_clone = conn_clone.clone();
                            let agent_io_clone = agent_io_clone.clone();

                            async move
                            {
                                agent_io_clone.cancel().await;

                                ctx.reply( Ok( () ) )
                            }

                        }
                    );
                }
            );

        cr.insert( BLUEZ_AGENT_PATH, &[iface_token], agent_ctx );

        self.conn.start_receive(
            MatchRule::new_method_call()
        ,   Box::new(
                move | msg, conn |
                {
                    log::debug!( "dbus msg {:?}", msg );
                    cr.handle_message( msg, conn ).unwrap();
                    true
                }
            )
        );

        let proxy = Proxy::new( BLUEZ_SERVICE_NAME, BLUEZ_AGENT_MANAGER_PATH, TIME_OUT, self.conn.clone() );

        let param = ( Path::from( BLUEZ_AGENT_PATH ), String::from( capability ) );

        match proxy.method_call::< (), _, _, _ >( BLUEZ_AGENT_MANAGER_INTERFACE, "RegisterAgent", param ).await
        {
            Ok( _ ) => {}
        ,   Err( x ) =>
            {
                log::error!( "dbus method_call error {:?} {:?} {:?}", BLUEZ_AGENT_MANAGER_INTERFACE, "RegisterAgent", x );
            }
        }

        let param = ( Path::from( BLUEZ_AGENT_PATH ), );

        match proxy.method_call::< (), _, _, _ >( BLUEZ_AGENT_MANAGER_INTERFACE, "RequestDefaultAgent", param ).await
        {
            Ok( _ ) => {}
        ,   Err( x ) =>
            {
                log::error!( "dbus method_call error {:?} {:?} {:?}", BLUEZ_AGENT_MANAGER_INTERFACE, "RequestDefaultAgent", x );
            }
        }

        self.agent = true;
    }
}

impl Drop for BtConn
{
    fn drop( &mut self )
    {
        log::debug!( "BtConn drop");

        if self.agent
        {
            let conn = self.conn.clone();

            tokio::spawn(
                async move
                {
                    let proxy = Proxy::new( BLUEZ_SERVICE_NAME, BLUEZ_AGENT_MANAGER_PATH, TIME_OUT, conn );

                    let param = ( Path::from( BLUEZ_AGENT_PATH ), );

                    match proxy.method_call::< (), _, _, _ >( BLUEZ_AGENT_MANAGER_INTERFACE, "UnregisterAgent", param ).await
                    {
                        Ok( _ ) => {}
                    ,   Err( x ) =>
                        {
                            log::error!( "dbus method_call error {:?} {:?} {:?}", BLUEZ_AGENT_MANAGER_INTERFACE, "UnregisterAgent", x );
                        }
                    }
                }
            );

            self.agent = false;
        }
    }
}

/// https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/adapter-api.txt
///
impl BtAdapter
{
    pub fn get_id( &self ) -> &str
    {
        &self.path
    }

    pub async fn get_status( &self, with_devices : bool  ) -> Result< BtAdapterStatus >
    {
        get_adapter_status( self.conn.clone(), &self.path, with_devices ).await
    }

    pub async fn set_alias( &self, value : &str ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "Alias", value ).await
    }

    pub async fn set_discoverable( &self, value : bool ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "Discoverable", value ).await
    }

    pub async fn set_discoverable_timeout( &self, value: u64 ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "DiscoverableTimeout", value ).await
    }

    pub async fn set_pairable( &self, value : bool ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "Pairable", value ).await
    }

    pub async fn set_pairable_timeout( &self, value: u64 ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "PairableTimeout", value ).await
    }

    pub async fn set_powered( &self, value : bool ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "Powered", value ).await
    }

    pub async fn start_discovery( &self ) -> Result< () >
    {
        call_void_func( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "StartDiscovery" ).await
    }

    pub async fn stop_discovery( &self ) -> Result< () >
    {
        call_void_func( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "StopDiscovery" ).await
    }

    pub async fn remove_device( &self, device: &str ) -> Result< () >
    {
        let device_path = dbus::strings::Path::from( device );
        call_void_func_a( self.conn.clone(), &self.path, BLUEZ_ADAPTER_INTERFACE, "RemoveDevice", device_path ).await
    }

    pub async fn get_devices( &self ) -> Result< Vec< BtDevice > >
    {
        let devices = get_device_path( self.conn.clone(), &self.path ).await?;
        Ok( devices.iter().map( | x | BtDevice { conn : self.conn.clone(), path : String::from( x ) } ).collect() )
    }

    pub async fn get_device( &self, path : &str ) -> Result< BtDevice >
    {
        let devices = get_device_path( self.conn.clone(), &self.path ).await?;

        if let Some( x ) = devices.iter().find( |x| *x == path )
        {
            Ok( BtDevice{ conn : self.conn.clone(), path : String::from( x ) } )
        }
        else
        {
            Err( dbus::Error::new_custom( "Error", "Bt device not found" ) )
        }
    }
}

/// https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/device-api.txt
///
impl BtDevice
{
    pub fn get_id( &self ) -> &str
    {
        &self.path
    }

    pub async fn get_status( &self ) -> Result< BtDeviceStatus >
    {
        get_device_status( self.conn.clone(), &self.path ).await
    }

    pub async fn set_trusted( &self, value : bool ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "Trusted", value ).await
    }

    pub async fn set_blocked( &self, value : bool ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "Blocked", value ).await
    }

    pub async fn set_wake_allowed( &self, value : bool ) -> Result< () >
    {
        set( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "WakeAllowed", value ).await
    }

    pub async fn connect( &self ) -> Result< () >
    {
        call_void_func( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "Connect" ).await
    }

    pub async fn disconnect( &self ) -> Result< () >
    {
        call_void_func( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "Disconnect" ).await
    }

    pub async fn connect_profile( &self, uuid : &str ) -> Result< () >
    {
        call_void_func_a( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "ConnectProfile", uuid ).await
    }

    pub async fn disconnect_profile( &self, uuid : &str ) -> Result< () >
    {
         call_void_func_a( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "DisconnectProfile", uuid ).await
    }

    pub async fn pair( &self ) -> Result< () >
    {
        call_void_func( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "Pair" ).await
    }

    pub async fn cancel_pairing( &self ) -> Result< () >
    {
        call_void_func( self.conn.clone(), &self.path, BLUEZ_DEVICE_INTERFACE, "CancelPairing" ).await
    }
}

