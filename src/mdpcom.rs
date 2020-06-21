
use std::fmt;
use std::sync::Mutex;

use actix_web::web;

use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::time::{ timeout, Duration, Instant };
use tokio::sync::{ oneshot, mpsc };
use tokio::prelude::*;

use serde::{ Serialize, /* Deserialize */ };

///
#[derive(Debug, Serialize, Clone)]
pub struct MpdComOk
{
    pub flds:       Vec<(String,String)>
,   pub bin:        Option<Vec<u8>>
}

///
#[derive(Debug, Serialize, Clone)]
pub struct MpdComErr
{
    pub err_code:   i32
,   pub cmd_index:  i32
,   pub cur_cmd:    String
,   pub msg_text:   String
}

///
impl fmt::Display for MpdComErr
{
    fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
    {
		write!( f, "code:{} msg:{}", self.err_code, self.msg_text )
    }
}

///
impl MpdComOk
{
	fn new() -> MpdComOk
	{
		MpdComOk { flds : Vec::new(), bin : None }
	}
}

///
impl MpdComErr
{
	fn new( err_code : i32 ) -> MpdComErr
	{
		MpdComErr{
	        err_code
	    ,   cmd_index   : 0
	    ,   cur_cmd     : String::new()
	    ,   msg_text    : String::new()
		}
	}
}

///
pub type MpdComResult = Result< MpdComOk, MpdComErr >;

///
pub struct MpdComRequest
{
	pub req	 : String
,	pub tx	 : oneshot::Sender< MpdComResult >
}

///
impl MpdComRequest
{
	pub fn new() -> ( MpdComRequest, oneshot::Receiver< MpdComResult > )
	{
		let ( tx, rx ) = oneshot::channel::< MpdComResult >();

		(
			MpdComRequest{
		        req			: String::new()
		    ,   tx
			}
		,	rx
		)
	}
}

///
pub fn quote_arg( arg: &str ) -> String
{
    let mut arg = String::from( arg.replace( '\\', r"\\" ).replace( '"', r#"\""# ) );

    if arg.contains( ' ' )
    {
        arg = String::from( "\"" ) + &arg + "\""
    }

    arg
}

///
async fn mpdcon_exec( cmd : String, conn : &mut TcpStream, protolog : bool )
-> Result< MpdComResult, Box< dyn std::error::Error> >
{
	if protolog
	{
		log::debug!( "> {}", cmd );
	}

	conn.write( cmd.as_bytes() ).await?;
	conn.write( &[0x0a] ).await?;
	conn.flush().await?;

	let mut is_ok = false;
	let mut ret_ok = MpdComOk::new();
	let mut ret_err = MpdComErr::new( -1 );

    let mut reader = BufReader::new( conn );
    let mut buf = String::new();

    'outer: loop
    {
        buf.clear();

        if let Ok( x ) = reader.read_line( &mut buf ).await
        {
			if x == 0
			{
				break 'outer;
			}
        }

		if protolog
		{
			log::debug!( "< {}", buf.trim_end() );
		}

        if buf == "OK\n"
        {
			is_ok = true;
			break 'outer;
        }
        else if buf.starts_with( "ACK [" )
        {
            lazy_static! {
                static ref RE: regex::Regex =
                    regex::Regex::new( r"^ACK\s*\[(\d+)@(\d+)\]\s+\{([^}]*)\}\s*(.*)\n" ).unwrap();
            }

            if let Some( x ) = RE.captures( &buf )
            {
				ret_err.err_code 	= x[1].parse().unwrap();
			    ret_err.cmd_index   = x[2].parse().unwrap();
			    ret_err.cur_cmd     = String::from( &x[3] );
			    ret_err.msg_text    = String::from( &x[4] );

				break 'outer;
            }
        }
        else
        {
            lazy_static! {
                static ref RE: regex::Regex =
                    regex::Regex::new( r"^([^:]*):\s*(.*)\n" ).unwrap();
            }

            if let Some( x ) = RE.captures( &buf )
            {
                if &x[1] == "binary"
                {
					let binlen : usize = x[2].parse().unwrap();
					let mut bin = Vec::<u8>::with_capacity( binlen );

					let mut buf = [0u8; 2048];

			        if let Ok( x ) = reader.read( &mut buf ).await
			        {
						if x == 0
						{
							break 'outer;
						}
						else
						{
							bin.extend_from_slice( &buf[0..x] );
						}
			        }

			        ret_ok.bin = Some( bin );
				}
				else
				{
                    ret_ok.flds.push(
                        (
                            String::from( x[1].trim() )
                        ,   String::from( x[2].trim() )
                        )
                    );
                }
			}
        }
	}

	if protolog && !is_ok
	{
		log::error!( "< {:?}", ret_err );
	}

	Ok( if is_ok { Ok( ret_ok ) } else { Err( ret_err ) } )
}

///
pub async fn mpdcom_task(
	ctx     : web::Data< Mutex< super::Context > >
,	mut rx  : mpsc::Receiver< MpdComRequest > )
-> Result< (), Box< dyn std::error::Error> >
{
	log::debug!( "mpdcom starting." );

	let mpd_addr;
	let mpd_protolog;
	{
		let ctx = &ctx.lock().unwrap();

		mpd_addr = String::from( &ctx.config.mpd_addr );
		mpd_protolog = ctx.config.mpd_protolog;
	};

	let mut conn : Option< TcpStream > = None;
	let mut conn_try_time : Option< Instant > = None;
	let 	conn_err_retry = Duration::from_secs( 10 );

    let mut _mpd_version : Option< String > = None;

	let rx_time_out = Duration::from_millis( 250 );

	let mut status_try_time : Option< Instant > = None;
	let status_time_out = Duration::from_millis( 200 );

	log::debug!( "mpdcom start. {:?} {}", mpd_addr, mpd_protolog );

	loop
	{
		if conn.is_none() &&
			(	conn_try_time.is_none()
			||	conn_try_time.unwrap().elapsed() > conn_err_retry
			)
		{
			// try connection

			conn_try_time = Some( Instant::now() );

			match TcpStream::connect( &mpd_addr ).await
			{
				Ok( mut x ) =>
				{
				    let mut reader = BufReader::new( &mut x );
				    let mut buf = String::new();

				    reader.read_line( &mut buf ).await?;

					log::info!( "connected {}", &buf );

				    if !buf.starts_with("OK MPD ")
				    {
						log::warn!( "connect shutdown" );
						x.shutdown( std::net::Shutdown::Both )?;
				    }
					else
					{
						conn = Some( x );
						_mpd_version = Some( String::from( buf[7..].trim() ) )
					}
				}
				Err( x ) =>
				{
					log::warn!( "connect error [{:?}]", x );
				}
			}
		}

		if conn.is_some() &&
			(	status_try_time.is_none()
			||	status_try_time.unwrap().elapsed() > status_time_out
			)
		{
			let mut status_ok = false;

			match mpdcon_exec( String::from( "status" ), conn.as_mut().unwrap(), false ).await
			{
				Ok(x) =>
				{
					match x
					{
						Ok(x) =>
						{
							let ctx = &mut ctx.lock().unwrap();

							ctx.mpd_status.clear();
							ctx.mpd_status.extend_from_slice( &x.flds );
							ctx.mpd_status_time = Some( chrono::Local::now() );

							status_try_time = Some( Instant::now() );

							status_ok = true;

							// log::debug!( "mpdcom status {:?}", &ctx.mpd_status_time );
						}
					,	Err(_) => {}
					}
				}
			,	Err(x) =>
				{
					log::warn!( "connection error [{:?}]", x );
					conn.as_mut().unwrap().shutdown();
					conn = None;
					conn_try_time = Some( Instant::now() );
				}
			}

			if !status_ok
			{
				let ctx = &mut ctx.lock().unwrap();

				ctx.mpd_status.clear();
				ctx.mpd_status_time = None;
			}
		}

		match timeout( rx_time_out, rx.recv() ).await
		{
			Ok(recv) =>
			{
				let recv = recv.unwrap();

				log::debug!( "rx recv [{}]", recv.req );

				if recv.req == "close"
				{
					if conn.is_some()
					{
						log::info!( "connection close" );
						conn.as_mut().unwrap().shutdown();
					}

					recv.tx.send( Ok( MpdComOk::new() ) ).ok();
					break;
				}
				else if conn.is_some()
				{
					match mpdcon_exec( recv.req, conn.as_mut().unwrap(), mpd_protolog ).await
					{
						Ok(x) =>
						{
							recv.tx.send( x ).ok();
						}
					,	Err(x) =>
						{
							log::warn!( "connection error [{:?}]", x );
							conn.as_mut().unwrap().shutdown();
							conn = None;
							conn_try_time = Some( Instant::now() );

							recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
						}
					}
				}
				else
				{
					recv.tx.send( Err( MpdComErr::new( -2 ) ) ).ok();
				}
			}
		,	Err(_) =>
			{
				// log::debug!( "{:?}", x );
			}
		}
	}

	log::debug!( "mpdcom stop." );

    Ok(())
}

