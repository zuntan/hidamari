//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::sync::Mutex;
use std::io::{ self, Read };
use std::collections::VecDeque;
use std::fs::File;

use std::os::unix::io::{ AsRawFd };
use libc::{F_GETFL, F_SETFL, fcntl, O_NONBLOCK};

use actix_web::web;

use tokio::time::{ delay_for, Duration, Instant };
use tokio::sync::{ mpsc };

use serde::{ Serialize, /* Deserialize */ };

use chfft::CFft1D;
use num_complex::Complex;

use crate::event;

#[derive(Debug, Serialize, Clone)]
struct SpecData
{
    spec_t : String
,   spec_l : Vec::< u32 >
,   spec_r : Vec::< u32 >
,   rms_l  : u32
,   rms_r  : u32
}

type SpecDataResult<'a> = Result< &'a SpecData, () >;

#[derive(Debug, Serialize, Clone)]
struct SpecHeader<'a>
{
    spec_h : &'a[ u32 ]
}

type SpecHeaderResult<'a> = Result< SpecHeader<'a>, () >;

fn open_fifo( fifo_name : &str ) -> io::Result< File >
{
    if fifo_name == ""
    {
        return Err( io::Error::from( io::ErrorKind::NotFound ) );
    }

    let fifo = File::open( fifo_name )?;

    let fd = fifo.as_raw_fd();

    let flags = unsafe { fcntl( fd, F_GETFL, 0 ) };

    if flags < 0
    {
        return Err( io::Error::last_os_error() );
    }

    let flags = flags | O_NONBLOCK;

    let res = unsafe { fcntl( fd, F_SETFL, flags ) };

    if res != 0
    {
        return Err( io::Error::last_os_error() );
    }

    Ok( fifo )
}


const SAMPLING_RATE     : usize = 44100;
const CHANNELS          : usize = 2;
const F_BUF_SIZE        : usize = SAMPLING_RATE / 20;
const F_BUF_SAMPLE_SZ   : usize = 2;
const S_BUF_SIZE        : usize = 8192;
const FIFO_ERROR_SLEEP  : Duration = Duration::from_millis( 100 );
const FIFO_STALL_SLEEP  : Duration = Duration::from_millis( 10 );
const FIFO_STALL_RESET  : Duration = Duration::from_millis( 50 );
const FIFO_STALL_REOPEN : Duration = Duration::from_millis( 1000 );
const FFT_BUF_SIZE      : usize = S_BUF_SIZE / 2;
const FFT_BUF_SLIDE_SIZE: usize = FFT_BUF_SIZE / 2;
const FFT_SPEC_SIZE     : usize = FFT_BUF_SIZE / 2;
const FFT_SPEC_HZ_D     : f32 = SAMPLING_RATE as f32 / 2.0 / FFT_SPEC_SIZE as f32;
const OCT_SCALE         : f32 = 2.0;
const ENABLE_CORRECTION : bool  = true;

pub async fn mpdfifo_task(
    ctx     : web::Data< Mutex< super::Context > >
,   mut rx  : mpsc::Receiver< event::EventRequest >
)
-> Result< (), Box< dyn std::error::Error> >
{
    let mut fft_engine_chfft = CFft1D::<f32>::with_len( FFT_BUF_SIZE );

    let mut f_buf = [ 0u8 ; F_BUF_SAMPLE_SZ * F_BUF_SIZE ];
    let     mut s_buf = VecDeque::< i16 >::with_capacity( S_BUF_SIZE );

    let mut fft_i_l : Vec::< Complex< f32 > > = vec![ Complex::new( 0.0, 0.0 ); FFT_BUF_SIZE ];
    let mut fft_i_r : Vec::< Complex< f32 > > = vec![ Complex::new( 0.0, 0.0 ); FFT_BUF_SIZE ];

    let mut fft_amp_l : Vec::< f32 > = vec![ 0.0 ; FFT_SPEC_SIZE ];
    let mut fft_amp_r : Vec::< f32 > = vec![ 0.0 ; FFT_SPEC_SIZE ];
    let mut fft_amp_b : Vec::< usize > = vec![ 0 ; FFT_SPEC_SIZE ];

    let spec_len     : usize = ( ( SAMPLING_RATE as f32 ).log2().floor() * OCT_SCALE ) as usize;

    let mut spec_amp_l : Vec::< f32 > = vec![ 0.0 ; spec_len ];
    let mut spec_amp_r : Vec::< f32 > = vec![ 0.0 ; spec_len ];
    let mut spec_amp_h : Vec::< u32 > = vec![ 0   ; spec_len ];
    let mut spec_amp_n : Vec::< f32 > = vec![ 0.0 ; spec_len ];
    let mut spec_amp_p : Vec::< f32 > = vec![ 0.0 ; spec_len ];

    let mut bar_st = 0;
    let mut bar_ed = 0;

    let mut s_buf_delay_size;

    let mut _fcnt = 0;
    let mut _rcnt = 0;
    let mut _scnt = 0;

    for i in 0..spec_len
    {
        let hz = 2_f32.powf( i as f32 / OCT_SCALE ) as u32;

        spec_amp_h[ i ] = hz;

        if bar_st == 0 && hz > 16
        {
            bar_st = i;
        }

        if bar_ed == 0 && hz >= 20000
        {
            bar_ed = i;
        }

        if ENABLE_CORRECTION
        {
            spec_amp_p[ i ] = i as f32 / OCT_SCALE / 4.0 * 10.0;
        }
        else
        {
            spec_amp_p[ i ] = 2.0 * 10.0;
        }
    }

    for i in 0..FFT_SPEC_SIZE
    {
        let hz = FFT_SPEC_HZ_D * ( i as f32 + 0.5 );
        let p = ( hz.log2() * OCT_SCALE ).floor() as usize;

        spec_amp_n[ p ] += 1.0;
        fft_amp_b[ i ] = p;
    }

    let mut spd = SpecData
    {
        spec_t : String::new()
    ,   spec_l : Vec::< u32 >::new()
    ,   spec_r : Vec::< u32 >::new()
    ,   rms_l  : 0
    ,   rms_r  : 0
    };

    for _ in bar_st..bar_ed
    {
        spd.spec_l.push( 0 );
        spd.spec_r.push( 0 );
    }

    let fifo_name =
    {
        let ctx = &mut ctx.lock().unwrap();

        let d = Duration::from_millis( ctx.config_dyn.mpdfifo_delay as u64 ).as_secs_f32();
        s_buf_delay_size = ( d * ( SAMPLING_RATE * CHANNELS ) as f32 ) as usize;

        let bh : SpecHeaderResult = Ok( SpecHeader { spec_h : &spec_amp_h[ bar_st..bar_ed ] } );

        if let Ok( x ) = serde_json::to_string( &bh )
        {
            ctx.spec_head_json = x;
        }

        String::from( &ctx.config.mpd_fifo )
    };

    if fifo_name != ""
    {
        log::info!( "mpdfifo enable target [{}]", &fifo_name );
    }
    else
    {
        log::info!( "mpdfifo disable" );
    }

    let mut fifo = open_fifo( &fifo_name );

    if let Err( ref x ) = fifo
    {
        if fifo_name != ""
        {
            log::error!( "mpdfifo error {:?}", x );
        }
    }

    let mut fifo_stall_time : Option< Instant > = None;
    let mut fifo_stall_reset = false;

    match fifo
    {
        Err(_) => {}
    ,   Ok( ref mut fifo_file ) =>
        {
            // pre read

            loop
            {
                match fifo_file.read( &mut f_buf )
                {
                    Err( ref x )
                        if  x.kind() == io::ErrorKind::WouldBlock
                    /*  ||  x.kind() == io::ErrorKind::Interrupted  */
                        =>
                    {
                        break;
                    }
                ,   Err( x ) =>
                    {
                        log::error!( "mpdfifo error {:?}", &x );
                        fifo = Err( x );
                        break;
                    }
                ,   Ok( _ ) => {
                        delay_for( FIFO_STALL_SLEEP ).await;
                    }
                }
            }
        }
    }

    macro_rules! update_ctx
    {
        () =>
        {
            let ctx = &mut ctx.lock().unwrap();

            let d = Duration::from_millis( ctx.config_dyn.mpdfifo_delay as u64 ).as_secs_f32();
            s_buf_delay_size = ( d * ( SAMPLING_RATE * CHANNELS ) as f32 ) as usize;

            spd.spec_t = chrono::Local::now().to_rfc3339();

            for i in bar_st..bar_ed
            {
                spd.spec_l[ i - bar_st ] = spec_amp_l[ i ] as u32;
                spd.spec_r[ i - bar_st ] = spec_amp_l[ i ] as u32;
            }

            let bd : SpecDataResult = Ok( &spd );

            if let Ok( x ) = serde_json::to_string( &bd )
            {
                ctx.spec_data_json = x;
            }
        }
    }

    macro_rules! fifo_reset
    {
        () =>
        {
            if let Some( x ) = fifo_stall_time
            {
                _scnt += 1;

                if !fifo_stall_reset && x.elapsed() > FIFO_STALL_RESET
                {
                    for p in 0..spec_len
                    {
                        spec_amp_l[ p ] = 0.0;
                        spec_amp_r[ p ] = 0.0;
                    }

                    spd.rms_l = 0;
                    spd.rms_r = 0;

                    fifo_stall_reset = true;

                    update_ctx!();
                }
                else if x.elapsed() > FIFO_STALL_REOPEN
                {
                    _rcnt += 1;

                    s_buf.clear();

                    fifo = open_fifo( &fifo_name );

                    if let Err( ref x ) = fifo
                    {
                        if fifo_name != ""
                        {
                            log::error!( "mpdfifo error {:?}", x );
                        }
                    }

                    fifo_stall_time = None;
                    fifo_stall_reset = false;
                }
            }
            else
            {
                fifo_stall_time = Some( Instant::now() );
            }

            for _ in 0..( FIFO_STALL_SLEEP.as_secs_f32() * ( SAMPLING_RATE * CHANNELS ) as f32 ) as usize
            {
                s_buf.pop_front();
                s_buf.push_back( 0 );
            }

            delay_for( FIFO_STALL_SLEEP ).await;
        }
    }

    loop
    {
        if event::event_shutdown( &mut rx ).await
        {
            break;
        }

        match fifo
        {
            Err(_) =>
            {
                delay_for( FIFO_ERROR_SLEEP ).await;
            }
        ,   Ok( ref mut fifo_file ) =>
            {
                match fifo_file.read( &mut f_buf )
                {
                    Err( ref x )
                        if  x.kind() == io::ErrorKind::WouldBlock
                    /*  ||  x.kind() == io::ErrorKind::Interrupted  */
                        =>
                    {
                        fifo_reset!();
                    }
                ,   Err( x ) =>
                    {
                        log::error!( "mpdfifo error {:?}", &x );
                        fifo = Err( x );
                    }
                ,   Ok( n ) =>
                    {
                        if n == 0
                        {
                            fifo_reset!();
                        }
                        else
                        {
                            if let Some( _ ) = fifo_stall_time
                            {
                                fifo_stall_time = None;
                                fifo_stall_reset = false;
                            }

                            for i in 0..n / 2
                            {
                                let mut b = [ 0u8 ; 2 ];

                                b[ 0 ] = f_buf[ i * CHANNELS ];
                                b[ 1 ] = f_buf[ i * CHANNELS + 1 ];

                                let x = i16::from_le_bytes( b );
                                s_buf.push_back( x );
                            }
                        }

                        if s_buf.len() > FFT_BUF_SIZE * CHANNELS + s_buf_delay_size
                        {
                            while s_buf.len() > FFT_BUF_SIZE * CHANNELS + s_buf_delay_size
                            {
                                s_buf.pop_front();
                            }

                            {
                                let mut s_buf_iter = s_buf.iter();

                                let mut sum_l : f32 = 0.0;
                                let mut sum_r : f32 = 0.0;

                                for i in 0..FFT_BUF_SIZE
                                {
                                    let l = *s_buf_iter.next().unwrap() as f32 / std::i16::MAX as f32;
                                    let r = *s_buf_iter.next().unwrap() as f32 / std::i16::MAX as f32;

                                    fft_i_l[ i ] = Complex::< f32 >::new( l, 0.0 );
                                    fft_i_r[ i ] = Complex::< f32 >::new( r, 0.0 );

                                    sum_l += l * l;
                                    sum_r += r * r;
                                }

                                spd.rms_l = ( ( sum_l / FFT_BUF_SIZE as f32 ).sqrt() * 1000.0 ) as u32;
                                spd.rms_r = ( ( sum_r / FFT_BUF_SIZE as f32 ).sqrt() * 1000.0 ) as u32;
                            }

                            for _ in 0.. ( FFT_BUF_SIZE - FFT_BUF_SLIDE_SIZE ) * CHANNELS
                            {
                                s_buf.pop_front();
                            }

                            let fft_o_l = fft_engine_chfft.forward( fft_i_l.as_slice() );
                            let fft_o_r = fft_engine_chfft.forward( fft_i_r.as_slice() );

                            for p in 0..spec_len
                            {
                                spec_amp_l[ p ] = 0.0;
                                spec_amp_r[ p ] = 0.0;
                            }

                            for i in 0..FFT_SPEC_SIZE
                            {
                                fft_amp_l[ i ] = fft_o_l[ i ].norm_sqr().sqrt().log10() * 20.0;
                                fft_amp_r[ i ] = fft_o_r[ i ].norm_sqr().sqrt().log10() * 20.0;

                                spec_amp_l[ fft_amp_b[ i ] ] += fft_amp_l[ i ];
                                spec_amp_r[ fft_amp_b[ i ] ] += fft_amp_r[ i ];
                            }

                            for p in 0..spec_len
                            {
                                if spec_amp_n[ p ] != 0.0
                                {
                                    spec_amp_l[ p ] /= spec_amp_n[ p ];
                                    spec_amp_r[ p ] /= spec_amp_n[ p ];
                                }

                                spec_amp_l[ p ] = ( spec_amp_l[ p ].max( 0.0 ) * spec_amp_p[ p ] ).min( 1000.0 );
                                spec_amp_r[ p ] = ( spec_amp_r[ p ].max( 0.0 ) * spec_amp_p[ p ] ).min( 1000.0 );
                            }

                            _fcnt += 1;

                            update_ctx!();
                        }
                    }
                }
            }
        }
    }

    log::debug!( "mpdfifo stop." );

    Ok(())
}

