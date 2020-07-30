//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::sync::{ Arc, Mutex };
use std::collections::HashMap;

use tokio::time::{ Duration };

use serde::{ Serialize, /* Deserialize */ };

use dbus_tokio::connection::{ self, IOResource };

use dbus::nonblock::{ SyncConnection, MsgMatch, Proxy, MethodReply, stdintf::org_freedesktop_dbus::Properties };
use dbus::message::{ MatchRule, MessageType };
use dbus::strings::{ BusName, Path, Interface, Member };
use dbus::arg::{ Variant, RefArg };
use dbus::tree::Factory;

type Result< T > = std::result::Result< T, Box< dyn std::error::Error > >;

static BLUEZ_SENDER                 : &'static str = "org.bluez";
static BLUEZ_SERVICE_NAME           : &'static str = "org.bluez";

static OBJECT_MANAGER_INTERFACE     : &'static str = "org.freedesktop.DBus.ObjectManager";
static GET_MANAGED_OBJECTS          : &'static str = "GetManagedObjects";

static BLUEZ_ADAPTER_INTERFACE      : &'static str = "org.bluez.Adapter1";
static BLUEZ_DEVICE_INTERFACE       : &'static str = "org.bluez.Device1";
static BLUEZ_AGENT_INTERFACE        : &'static str = "org.bluez.Agent1";

static AUDIO_SOURCE_UUID            : &'static str = "0000110a-0000-1000-8000-00805f9b34fb";
static AUDIO_SINK_UUID              : &'static str = "0000110b-0000-1000-8000-00805f9b34fb";

const TIME_OUT                      : Duration = Duration::from_secs( 3 );

type GetManagedObjectsRetType<'a> =
    HashMap< dbus::strings::Path<'a>, HashMap< String, HashMap< String, Variant< Box< dyn RefArg > > > > >;

type GetAllRetType = HashMap< String, Variant< Box< dyn RefArg > > >;

pub struct BtConn
{
    conn        : Arc< SyncConnection >
,   res_err     : Arc< Mutex< Option< String > > >
,   dump_mg     : bool
}

pub struct BtAdapter<'a>
{
    bt          : &'a BtConn
,   path        : String
}

#[derive(Debug, Serialize)]
pub struct BtAdapterStatus
{
    id                  : String
,   address             : String
,   address_type        : String
,   alias               : String
,   class               : u32
,   discoverable        : bool
,   discoverable_timeout : u32
,   discovering         : bool
,   modalias            : Option< String >
,   name                : String
,   pairable            : bool
,   pairable_timeout    : u32
,   powered             : bool
,   uuids               : Vec< String >
}

pub struct BtDevice<'a>
{
    bt          : &'a BtConn
,   path        : String
}

#[derive(Debug, Serialize)]
pub struct BtDeviceStatus
{
    id              : String
,   adapter         : String
,   address         : String
,   address_type    : String
,   alias           : String
,   appearance      : Option< i16 >
,   blocked         : bool
,   class           : u32
,   connected       : bool
,   icon            : String
,   legacy_pairing  : bool
,   modalias        : Option< String >
,   name            : String
,   paired          : bool
,   rssi            : Option< i16 >
,   services_resolved : bool
,   trusted         : bool
,   tx_power        : Option< i16 >
,   uuids           : Vec< String >
,   audio_source    : bool
,   audio_sink      : bool
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

        Ok( BtConn{ conn, res_err, dump_mg : true } )
    }

    pub async fn get_managed_objects<'a>( &self ) -> Result< GetManagedObjectsRetType<'a> >
    {
        let proxy = Proxy::new( BLUEZ_SERVICE_NAME, "/", TIME_OUT, self.conn.clone() );

        match proxy.method_call::< ( GetManagedObjectsRetType, ), _, _, _ >( OBJECT_MANAGER_INTERFACE, GET_MANAGED_OBJECTS, () ).await
        {
            Ok( x ) =>
            {
                if self.dump_mg
                {
                    log::debug!( "{}", &Self::pretty_dump_managed_objects( &x.0 ) );
                }

                Ok( x.0 )
            }
        ,   Err( x ) =>
            {
                log::debug!( "{:?}", x );
                Err( Box::new( x ) )
            }
        }
    }

    pub fn pretty_dump_managed_objects<'a>( mo : & GetManagedObjectsRetType<'a> ) -> String
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

    pub async fn get_adapter_path( &self ) -> Result< Vec< String > >
    {
        let mut ret = Vec::<String>::new();

        let mo = self.get_managed_objects().await?;

        for ( k, v ) in mo.iter()
        {
            if v.contains_key( BLUEZ_ADAPTER_INTERFACE )
            {
                ret.push( k.to_string() );
            }
        }

        Ok( ret )
    }

    pub async fn get_device_path( &self, adapter_path : &str ) -> Result< Vec< String > >
    {
        let mut ret = Vec::<String>::new();

        let mo = self.get_managed_objects().await?;

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

    pub async fn get_adapter<'a>( &'a self, path : &str ) -> Result< BtAdapter<'a> >
    {
        let paths = self.get_adapter_path().await?;

        if let Some( x ) = paths.iter().find( |x| *x == path )
        {
            Ok( BtAdapter{ bt : self, path : String::from( path ) } )
        }
        else
        {
            Err( Box::from( "Bt adapter not found" ) )
        }
    }

    pub async fn get_first_adapter<'a>( &'a self ) -> Result< BtAdapter<'a> >
    {
        let paths = self.get_adapter_path().await?;

        if !paths.is_empty()
        {
            Ok( BtAdapter{ bt : self, path : String::from( paths.first().unwrap() ) } )
        }
        else
        {
            Err( Box::from( "Bt adapter not found" ) )
        }
    }

    pub async fn call_void_func( &self, path : &str, interface : &str, func_name : &str ) -> Result< () >
    {
        let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, self.conn.clone() );

        match proxy.method_call::< (), _, _, _ >( interface, func_name, () ).await
        {
            Ok( _ ) =>
            {
                Ok( () )
            }
        ,   Err( x ) =>
            {
                log::debug!( "{:?}", x );
                Err( Box::new( x ) )
            }
        }
    }

    pub async fn call_void_func_a< T : dbus::arg::Arg + dbus::arg::Append >
        ( &self, path : &str, interface : &str, func_name : &str, value : T )
        -> Result< () >
    {
        let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, self.conn.clone() );

        match proxy.method_call::< (), _, _, _ >( interface, func_name, ( value, ) ).await
        {
            Ok( _ ) =>
            {
                Ok( () )
            }
        ,   Err( x ) =>
            {
                log::debug!( "{:?}", x );
                Err( Box::new( x ) )
            }
        }
    }

    pub async fn set< T : dbus::arg::Arg + dbus::arg::Append >
        ( &self, path : &str, interface : &str, key : &str, value : T )
        -> Result< () >
    {
        let proxy = Proxy::new( BLUEZ_SERVICE_NAME, path, TIME_OUT, self.conn.clone() );

        match proxy.set( interface, key, value ).await
        {
            Ok( _ ) =>
            {
                Ok( () )
            }
        ,   Err( x ) =>
            {
                log::debug!( "{:?}", x );
                Err( Box::new( x ) )
            }
        }
    }
}

/// https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/adapter-api.txt
///
impl <'a> BtAdapter<'a>
{
    pub fn get_id( &self ) -> &str
    {
        &self.path
    }

    pub async fn get_status( &self ) -> Result< BtAdapterStatus >
    {
        let proxy = Proxy::new( BLUEZ_SERVICE_NAME, &self.path, TIME_OUT, self.bt.conn.clone() );

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
                        id          : String::from( &self.path )
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
                    }
                )
            }
        ,   Err( x ) =>
            {
                log::debug!( "{:?}", x );
                Err( Box::new( x ) )
            }
        }
    }

    pub async fn set_alias( &self, value : &str ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_ADAPTER_INTERFACE, "Alias", value ).await
    }

    pub async fn set_discoverable( &self, value : bool ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_ADAPTER_INTERFACE, "Discoverable", value ).await
    }

    pub async fn set_discoverable_timeout( &self, value: u64 ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_ADAPTER_INTERFACE, "DiscoverableTimeout", value ).await
    }

    pub async fn set_pairable( &self, value : bool ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_ADAPTER_INTERFACE, "Pairable", value ).await
    }

    pub async fn set_pairable_timeout( &self, value: u64 ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_ADAPTER_INTERFACE, "PairableTimeout", value ).await
    }

    pub async fn set_powered( &self, value : bool ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_ADAPTER_INTERFACE, "Powered", value ).await
    }

    pub async fn start_discovery( &self ) -> Result< () >
    {
        self.bt.call_void_func( &self.path, BLUEZ_ADAPTER_INTERFACE, "StartDiscovery" ).await
    }

    pub async fn stop_discovery( &self ) -> Result< () >
    {
        self.bt.call_void_func( &self.path, BLUEZ_ADAPTER_INTERFACE, "StopDiscovery" ).await
    }

    pub async fn remove_device( &self, device: String ) -> Result< () >
    {
        let device_path = dbus::strings::Path::from( device );
        self.bt.call_void_func_a( &self.path, BLUEZ_ADAPTER_INTERFACE, "RemoveDevice", device_path ).await
    }

    pub async fn get_devices( &'a self ) -> Result< Vec< BtDevice<'a> > >
    {
        let devices = self.bt.get_device_path( &self.path ).await?;
        Ok( devices.iter().map( | x | BtDevice { bt : self.bt, path : String::from( x ) } ).collect() )
    }
}

/// https://git.kernel.org/pub/scm/bluetooth/bluez.git/tree/doc/device-api.txt
///
impl <'a> BtDevice<'a>
{
    pub fn get_id( &self ) -> &str
    {
        &self.path
    }

    pub async fn get_status( &self ) -> Result< BtDeviceStatus >
    {
        let proxy = Proxy::new( BLUEZ_SERVICE_NAME, &self.path, TIME_OUT, self.bt.conn.clone() );

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
                let class           = *props.get( "Class" ).unwrap().0.as_any().downcast_ref::<u32>().unwrap();
                let connected       = *props.get( "Connected" ).unwrap().0.as_any().downcast_ref::<bool>().unwrap();
                let icon            = String::from( props.get( "Icon" ).unwrap().0.as_str().unwrap() );
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

                let name            = String::from( props.get( "Name" ).unwrap().0.as_str().unwrap() );
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

                let uuids : Vec< String > = props.get( "UUIDs" ).unwrap().0.as_any().downcast_ref::< Vec<String> >().unwrap().iter().map( | x | String::from( x ) ).collect();

                let audio_source    = uuids.iter().find( | &x | x == AUDIO_SOURCE_UUID ).is_some();
                let audio_sink      = uuids.iter().find( | &x | x == AUDIO_SINK_UUID ).is_some();

                Ok(
                    BtDeviceStatus
                    {
                        id              : String::from( &self.path )
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
                    ,   audio_source
                    ,   audio_sink
                    }
                )
            }
        ,   Err( x ) =>
            {
                log::debug!( "{:?}", x );
                Err( Box::new( x ) )
            }
        }
    }

    pub async fn set_trusted( &self, value : bool ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_DEVICE_INTERFACE, "Trusted", value ).await
    }

    pub async fn set_blocked( &self, value : bool ) -> Result< () >
    {
        self.bt.set( &self.path, BLUEZ_DEVICE_INTERFACE, "Blocked", value ).await
    }

    pub async fn connect( &self ) -> Result< () >
    {
        self.bt.call_void_func( &self.path, BLUEZ_DEVICE_INTERFACE, "Connect" ).await
    }

    pub async fn disconnect( &self ) -> Result< () >
    {
        self.bt.call_void_func( &self.path, BLUEZ_DEVICE_INTERFACE, "Disconnect" ).await
    }

    pub async fn connect_profile( &self, uuid : &str ) -> Result< () >
    {
        self.bt.call_void_func_a( &self.path, BLUEZ_DEVICE_INTERFACE, "ConnectProfile", uuid ).await
    }

    pub async fn disconnect_profile( &self, uuid : &str ) -> Result< () >
    {
        self.bt.call_void_func_a( &self.path, BLUEZ_DEVICE_INTERFACE, "DisconnectProfile", uuid ).await
    }

    pub async fn pair( &self ) -> Result< () >
    {
        self.bt.call_void_func( &self.path, BLUEZ_DEVICE_INTERFACE, "Pair" ).await
    }

    pub async fn cancel_pairing( &self ) -> Result< () >
    {
        self.bt.call_void_func( &self.path, BLUEZ_DEVICE_INTERFACE, "CancelPairing" ).await
    }
}

