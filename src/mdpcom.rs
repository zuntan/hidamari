
use std::io;
use std::io::prelude::*;
use std::fmt;

use std::str::FromStr;
use std::sync::{ Arc, Mutex };

use actix_web::web;

use tokio::io::{ BufReader };
use tokio::net::{ TcpStream };
use tokio::time::{ delay_for, timeout, Duration, Instant };
use tokio::sync::{ oneshot, mpsc };
use tokio::prelude::*;

struct MpdComOk
{
    flds:       Vec<(String,String)>
,   bin:        Option<Vec<u8>>
}

#[derive(Debug)]
struct MpdComErr
{
    err_code:   i32
,   cmd_index:  i32
,   cur_cmd:    String
,   msg_text:   String
}

impl fmt::Display for MpdComErr
{
    fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
    {
		write!( f, "code:{} msg:{}", self.err_code, self.msg_text )
    }
}

type MpdComResult = Result< MpdComOk, MpdComErr >;

struct MpdComRequest
{
	cmd	: String
,	tx	: oneshot::Sender< MpdComResult >
}

fn quote_arges( arg: &str ) -> String
{
    let mut arg = String::from( arg.replace( '\\', r"\\" ).replace( '"', r#"\""# ) );

    if arg.contains( ' ' )
    {
        arg = String::from( "\"" ) + &arg + "\""
    }

    arg
}

async fn modComExec( cmd : String, conn : &mut TcpStream, protolog : bool )
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
	let mut ret_ok = MpdComOk { flds : Vec::new(), bin : None };
	let mut ret_err = MpdComErr{
        err_code    : -1
    ,   cmd_index   : 0
    ,   cur_cmd     : String::new()
    ,   msg_text    : String::new()
	};

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
			log::debug!( "< {}", buf );
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

					reader.read( &mut buf );

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

//	Ok( if is_ok { Ok( ret_ok ) } else { Err( ret_err ) } )
	Ok( Err( ret_err ) )
}

async fn modComTask(
	ctx     : web::Data< Mutex< super::Context > >
,	mut rx  : mpsc::Receiver< MpdComRequest > )
-> Result< (), Box< dyn std::error::Error> >
{
	let mpd_addr;
	let mpd_protolog;
	{
		let ctx = &ctx.lock().unwrap();

		mpd_addr = std::net::SocketAddr::from_str( &ctx.config.mpd_addr ).unwrap();
		mpd_protolog = ctx.config.mpd_protolog;
	};

	let mut conn : Option< TcpStream > = None;
	let mut conn_try_time : Option< Instant > = None;
	let 	conn_err_retry = Duration::from_secs( 10 );
    let mut mpd_version : Option< String > = None;

	let rx_time_out = Duration::from_millis( 100 );

	let mut status_try_time : Option< Instant > = None;
	let status_time_out = Duration::from_millis( 200 );

	loop
	{
		if conn.is_none() &&
			(	conn_try_time.is_none()
			||	conn_try_time.unwrap().elapsed() > conn_err_retry
			)
		{
			// try connection

			conn_try_time = Some( Instant::now() );

			match TcpStream::connect( mpd_addr ).await
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
						mpd_version = Some( String::from( buf[7..].trim() ) )
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
			match modComExec( String::from( "status" ), &mut conn.unwrap(), false ).await?
			{
				Ok(x) =>
				{
					let ctx = &mut ctx.lock().unwrap();

					ctx.mpd_status.clear();
					ctx.mpd_status.extend_from_slice( &x.flds );
					ctx.mpd_status_time = Some( Instant::now() );
				}
			,	Err(x) =>
				{
					log::error!( "tcp error : [{:?}]", &x )
				}
			}
		}


		let recv =  timeout( rx_time_out, rx.recv() );

		break;
	}

    Ok(())
}
