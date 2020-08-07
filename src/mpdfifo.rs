//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io::{ self, Read };
use std::collections::{ VecDeque, HashMap };
use std::fs::File;

use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{ AsRawFd };
use libc;

use tokio::time::{ delay_for, timeout, Duration, Instant };
use tokio::sync::{ mpsc, oneshot };

use serde::{ Serialize, Deserialize };

use chfft::CFft1D;
use num_complex::Complex;

use alsa::{ Direction, ValueOr };
use alsa::pcm::{ PCM, HwParams, Format, Access /*, State */ };

use crate::context;
use crate::event;
use crate::asyncread;

#[derive(Debug, Serialize, Clone)]
pub struct MpdfifoOk
{
    pub enable      : Option< bool >
}

impl MpdfifoOk
{
    fn new() -> MpdfifoOk
    {
        MpdfifoOk { enable : None }
    }
}


#[derive(Debug, Serialize, Clone)]
pub struct MpdfifoErr
{
    pub err_code : i32
,   pub err_msg : String
}

impl MpdfifoErr
{
    fn new( err_code : i32, err_msg : &str ) -> MpdfifoErr
    {
        MpdfifoErr{ err_code, err_msg : String::from( err_msg ) }
    }
}

///
pub type MpdfifoResult       = Result< MpdfifoOk, MpdfifoErr >;

#[derive(Debug)]
pub enum MpdfifoRequestType
{
    Nop
,   AlsaEnable( String, bool )
,   AlsaIsEnable( String )
,   Shutdown
}

///
pub struct MpdfifoRequest
{
    pub req  : MpdfifoRequestType
,   pub tx   : oneshot::Sender< MpdfifoResult >
}

impl MpdfifoRequest
{
    pub fn new() -> ( MpdfifoRequest, oneshot::Receiver< MpdfifoResult > )
    {
        let ( tx, rx ) = oneshot::channel::< MpdfifoResult >();

        (
            MpdfifoRequest{
                req         : MpdfifoRequestType::Nop
            ,   tx
            }
        ,   rx
        )
    }
}

#[derive(Debug, Serialize, Clone)]
struct SpecData
{
    spec_t : String
,   spec_l : Vec::< u32 >
,   spec_r : Vec::< u32 >
,   rms_l  : u32
,   rms_r  : u32
,   peak_l : u32
,   peak_r : u32
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

    let mut options = OpenOptions::new();

    options.read( true );
    options.custom_flags( libc::O_NONBLOCK | libc::O_RDONLY );

    let fifo = options.open( fifo_name )?;

    let fd = fifo.as_raw_fd();

    let flags = unsafe { libc::fcntl( fd, libc::F_GETFL, 0 ) };

    if flags < 0
    {
        return Err( io::Error::last_os_error() );
    }

    let flags = flags | libc::O_NONBLOCK;

    let res = unsafe { libc::fcntl( fd, libc::F_SETFL, flags ) };

    if res != 0
    {
        return Err( io::Error::last_os_error() );
    }

    Ok( fifo )
}

const FIFO_ERROR_SLEEP  : Duration = Duration::from_millis( 1000 );
const FIFO_STALL_SLEEP  : Duration = Duration::from_millis( 15 );
const FIFO_STALL_RESET  : Duration = Duration::from_millis( 60 );
const FIFO_STALL_REOPEN : Duration = Duration::from_millis( 1000 );

const SAMPLING_RATE     : usize = 44100;
const CHANNELS          : usize = 2;
const F_BUF_SIZE        : usize = 16384;

const ENABLE_CORRECTION : bool  = true;
const CORRECTION_1      : f32 = 4.0;
const CORRECTION_2      : f32 = 10.0;
const CORRECTION_3      : f32 = 20.0;

#[derive(Debug, Deserialize, Clone)]
struct AlsaParam
{
    a_buffer_t  : Option<u32>           // u sec    1 sec = 1000_000 u sec
,   a_period_t  : Option<u32>           // u sec    1 sec = 1000_000 u sec
}

fn split_alsa_dev_param<'a>( dev_param : &'a str ) -> ( &'a str, AlsaParam )
{
    let dev_param =
        if let Some( x ) = dev_param.strip_prefix( context::ALSA_SINK_PROTO )
        {
            x
        }
        else
        {
            dev_param
        };

    let v: Vec< &str > = dev_param.splitn( 2, "?" ).collect();

    let dev = v[0];

    let mut param = AlsaParam { a_buffer_t : None, a_period_t : None };

    if v.len() >= 2
    {
        if let Ok( x ) = serde_urlencoded::from_str::< AlsaParam >( v[1] )
        {
            param = x;
        }
    }

    ( dev, param )
}

struct AlsaOutput
{
    pcm         : PCM
,   buf         : VecDeque< i16 >
,   buf_flag    : bool
}

pub const DEFALUT_A_BUFFER_T        : u32 = asyncread::DEFALUT_A_BUFFER_T;
pub const DEFALUT_A_PERIOD_T        : u32 = asyncread::DEFALUT_A_PERIOD_T;

pub const ALSA_INIT_BUFFERING_SIZE  : usize = 4410;

fn open_alsa( dev : &str, mut param : AlsaParam ) -> io::Result< AlsaOutput >
{
    if param.a_buffer_t.is_none()
    {
        param.a_buffer_t = Some( DEFALUT_A_BUFFER_T );
    }

    if param.a_period_t.is_none()
    {
        param.a_period_t = Some( DEFALUT_A_PERIOD_T );
    }

    match PCM::new( &dev, Direction::Playback, true )
    {
        Ok( pcm ) =>
        {
            {
                let hwp = HwParams::any( &pcm ).unwrap();

                if let Err( x ) = hwp.set_rate( SAMPLING_RATE as u32, ValueOr::Nearest )
                {
                    log::error!( "Alsa hwp.set_rate error. {:?}", x );
                }

                if let Err( x ) = hwp.set_channels( CHANNELS as u32 )
                {
                    log::error!( "Alsa hwp.set_channels error. {:?}", x );
                }

                if let Err( x ) = hwp.set_format( Format::s16() )
                {
                    log::error!( "Alsa hwp.set_format error. {:?}", x );
                }

                if let Some( x ) = param.a_buffer_t
                {
                    if let Err( x ) = hwp.set_buffer_time_near( x, ValueOr::Nearest )
                    {
                        log::error!( "Alsa hwp.set_buffer_time_near error. {:?}", x );
                    }
                }

                if let Some( x ) = param.a_period_t
                {
                    if let Err( x ) = hwp.set_period_time_near( x, ValueOr::Nearest )
                    {
                        log::error!( "Alsa hwp.set_period_time_near error. {:?}", x );
                    }
                }

                if let Err( x ) = hwp.set_access( Access::RWInterleaved )
                {
                    log::error!( "Alsa hwp.set_access error. {:?}", x );
                }

                if let Err( x ) = pcm.hw_params( &hwp )
                {
                    log::error!( "Alsa hw_params error. {:?}", x );

                    return Err( io::Error::new( io::ErrorKind::ConnectionRefused, x ) );
                }

                if log::log_enabled!( log::Level::Debug )
                {
                    let rate    = hwp.get_rate().unwrap();
                    let ch      = hwp.get_channels().unwrap();
                    let fmt     = hwp.get_format().unwrap();
                    let b_size  = hwp.get_buffer_size().unwrap();
                    let p_size  = hwp.get_period_size().unwrap();

                    log::debug!(
                        "ALSA HWP rate:{:?} channels:{:?} format:{:?} buffer_time:{:?} period_time:{:?}"
                    ,   rate
                    ,   ch
                    ,   fmt
                    ,   b_size as f32 / rate as f32
                    ,   p_size as f32 / rate as f32
                    );
                }
            }

            if let Err( x ) = pcm.prepare()
            {
                log::error!( "AlsaCaptureLameEncode start error. {:?}", x );

                Err( io::Error::new( io::ErrorKind::ConnectionRefused, x ) )
            }
            else
            {
                Ok( AlsaOutput{ pcm, buf : VecDeque::new(), buf_flag : false } )
            }
        }
    ,   Err( x ) =>
        {
            log::error!( "Alsa open error. {:?}", x );

            Err( io::Error::new( io::ErrorKind::NotFound, x ) )
        }
    }
}

pub async fn mpdfifo_task(
    arwlctx : context::ARWLContext
,   mut rx  : mpsc::Receiver< MpdfifoRequest >
)
-> io::Result< ()  >
{
    let mut fftmode : u32;

    {
        let ctx = arwlctx.read().await;

        fftmode = ctx.config.mpd_fifo_fftmode;
    }

    let fft_buf_size      : usize;
    let fft_buf_slide_size: usize;
    let oct_scale         : f32;

    match fftmode
    {
        5 =>
        {
            fft_buf_size        = 2048;
            fft_buf_slide_size  = 2048 * 3;
            oct_scale           = 1.0;
        }
    ,   4 =>
        {
            fft_buf_size        = 2048;
            fft_buf_slide_size  = 2048 * 2;
            oct_scale           = 1.0;
        }
    ,   3 =>
        {
            fft_buf_size        = 2048;
            fft_buf_slide_size  = 2048 * 1;
            oct_scale           = 1.0;
        }
    ,   2 =>
        {
            fft_buf_size        = 4096;
            fft_buf_slide_size  = 2048 * 3;
            oct_scale           = 2.0;
        }
    ,   1 =>
        {
            fft_buf_size        = 4096;
            fft_buf_slide_size  = 2048 * 2;
            oct_scale           = 2.0;
        }
    ,   _ =>
        {
            fft_buf_size        = 4096;
            fft_buf_slide_size  = 2048 * 1;
            oct_scale           = 2.0;

            fftmode = 0;
        }
    }

    log::info!( "fftmode: {} / fft_buf_size: {} / fft_buf_slide_size: {} ({:.2}fps) / oct_scale: {}"
        ,   fftmode
        ,   fft_buf_size
        ,   fft_buf_slide_size
        ,   SAMPLING_RATE as f32 / fft_buf_slide_size as f32
        ,   oct_scale
    );

    let fft_spec_size     : usize   = fft_buf_size / 2;
    let fft_spec_hz_d     : f32     = SAMPLING_RATE as f32 / 2.0 / fft_spec_size as f32;

    let mut fft_engine_chfft = CFft1D::<f32>::with_len( fft_buf_size );

    let mut f_buf = [ 0u8  ;  F_BUF_SIZE ];
    let mut a_buf = [ 0i16 ;  F_BUF_SIZE / 2];
    let mut s_buf = VecDeque::< i16 >::with_capacity( fft_buf_size * 2 );

    let mut fft_i_l     : Vec::< Complex< f32 > > = vec![ Complex::new( 0.0, 0.0 ); fft_buf_size ];
    let mut fft_i_r     : Vec::< Complex< f32 > > = vec![ Complex::new( 0.0, 0.0 ); fft_buf_size ];

    let mut fft_amp_l   : Vec::< f32 > = vec![ 0.0 ; fft_spec_size ];
    let mut fft_amp_r   : Vec::< f32 > = vec![ 0.0 ; fft_spec_size ];
    let mut fft_amp_b   : Vec::< usize > = vec![ 0 ; fft_spec_size ];

    let spec_len        : usize = ( ( SAMPLING_RATE as f32 ).log2().floor() * oct_scale ) as usize;

    let mut spec_amp_l  : Vec::< f32 > = vec![ 0.0 ; spec_len ];
    let mut spec_amp_r  : Vec::< f32 > = vec![ 0.0 ; spec_len ];
    let mut spec_amp_h  : Vec::< u32 > = vec![ 0   ; spec_len ];
    let mut spec_amp_n  : Vec::< f32 > = vec![ 0.0 ; spec_len ];
    let mut spec_amp_p  : Vec::< f32 > = vec![ 0.0 ; spec_len ];

    let mut bar_st = 0;
    let mut bar_ed = 0;

    let mut s_buf_delay_size;

    let mut _fcnt = 0;
    let mut _rcnt = 0;
    let mut _scnt = 0;

    for i in 0..spec_len
    {
        let hz = 2_f32.powf( i as f32 / oct_scale ) as u32;

        spec_amp_h[ i ] = hz;

        if bar_st == 0 && hz > 16
        {
            bar_st = i;
        }

        if bar_ed == 0 && hz > 16384
        {
            bar_ed = i;
        }

        if ENABLE_CORRECTION
        {
            spec_amp_p[ i ] = i as f32 / oct_scale / CORRECTION_1 * CORRECTION_2;
        }
        else
        {
            spec_amp_p[ i ] = 2.0 * 10.0;
        }
    }

    if bar_ed < bar_st
    {
        bar_ed = bar_st.max( spec_len );
    }

    for i in 0..fft_spec_size
    {
        let hz = fft_spec_hz_d * ( i as f32 + 0.5 );
        let p = ( hz.log2() * oct_scale ).floor() as usize;

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
    ,   peak_l : 0
    ,   peak_r : 0
    };

    for _ in bar_st..bar_ed
    {
        spd.spec_l.push( 0 );
        spd.spec_r.push( 0 );
    }

    let fifo_name =
    {
        let mut ctx = arwlctx.write().await;

        let d = Duration::from_millis( ctx.config_dyn.spec_delay as u64 ).as_secs_f32();
        s_buf_delay_size = ( d * ( SAMPLING_RATE * CHANNELS ) as f32 ) as usize;

        let bh : SpecHeaderResult = Ok( SpecHeader { spec_h : &spec_amp_h[ bar_st..bar_ed ] } );

        if let Ok( x ) = serde_json::to_string( &bh )
        {
            ctx.spec_head_json = x;
        }

        spd.spec_t = chrono::Local::now().to_rfc3339();

        let bd : SpecDataResult = Ok( &spd );

        if let Ok( x ) = serde_json::to_string( &bd )
        {
            ctx.spec_data_json = x;
        }

        log::info!( "spec_head : {}", &ctx.spec_head_json );

        ctx.ws_data_intv = Duration::from_millis( ( 1000.0 * ( fft_buf_slide_size as f32 / SAMPLING_RATE as f32 ) ) as u64 );
        ctx.spec_enable = ctx.config.mpd_fifo != "";

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
                ,   Ok( x ) => {
                        if x == 0
                        {
                            break;
                        }
                    }
                }
            }
        }
    }

    macro_rules! update_ctx
    {
        () =>
        {
            let mut ctx = arwlctx.write().await;

            let d = Duration::from_millis( ctx.config_dyn.spec_delay as u64 ).as_secs_f32();
            s_buf_delay_size = ( d * ( SAMPLING_RATE * CHANNELS ) as f32 ) as usize;

            spd.spec_t = chrono::Local::now().to_rfc3339();

            for i in bar_st..bar_ed
            {
                spd.spec_l[ i - bar_st ] = spec_amp_l[ i ] as u32;
                spd.spec_r[ i - bar_st ] = spec_amp_r[ i ] as u32;
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
                    spd.peak_l = 0;
                    spd.peak_r = 0;

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

            let len = ( FIFO_STALL_SLEEP.as_secs_f32() * ( SAMPLING_RATE * CHANNELS ) as f32 ) as usize;
            let lend = s_buf.len().min( len );

            s_buf.drain( 0..lend );
            s_buf.extend( std::iter::repeat( 0 ).take( len ) );

            delay_for( FIFO_STALL_SLEEP ).await;
        }
    }

    // alsa

    let mut alsa_open_devices : HashMap< String, AlsaOutput > = HashMap::new();

    log::debug!( "mpdfifo start." );

    loop
    {
        match timeout( event::EVENT_WAIT_TIMEOUT, rx.recv() ).await
        {
            Ok( recv ) =>
            {
                let recv = recv.unwrap();

                if let MpdfifoRequestType::AlsaIsEnable( ref _dev_param ) = recv.req
                {
                }
                else
                {
                    log::debug!( "recv [{:?}]", recv.req );
                }

                match recv.req
                {
                    MpdfifoRequestType::Shutdown =>
                    {
                        recv.tx.send( Ok( MpdfifoOk::new() ) ).ok();
                        break;
                    }

                ,   MpdfifoRequestType::AlsaEnable( dev_param, sw ) =>
                    {
                        let ( dev, param ) = split_alsa_dev_param( &dev_param );

                        if sw && !alsa_open_devices.contains_key( dev )
                        {
                            match open_alsa( dev, param )
                            {
                                Ok( ao ) =>
                                {
                                    alsa_open_devices.insert( String::from( dev ), ao );
                                    recv.tx.send( Ok( MpdfifoOk::new() ) ).ok();
                                }
                            ,   Err( x ) =>
                                {
                                    recv.tx.send( Err( MpdfifoErr::new( -1, &format!( "Alsa open error [{:?}]", x ) ) ) ).ok();
                                }
                            }
                        }
                        else if !sw
                        {
                            let _ = alsa_open_devices.remove( dev );
                            recv.tx.send( Ok( MpdfifoOk::new() ) ).ok();
                        }
                    }

                ,   MpdfifoRequestType::AlsaIsEnable( dev_param ) =>
                    {
                        let ( dev, _ ) = split_alsa_dev_param( &dev_param );

                        let enable = Some( alsa_open_devices.contains_key( dev ) );

                        recv.tx.send( Ok( MpdfifoOk{ enable } ) ).ok();
                    }

                ,   _ => {}
                }
            }
        ,   Err( _ ) =>
            {
            }
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

                            let mut b = [ 0u8 ; 2 ];

                            let do_alsa = !alsa_open_devices.is_empty();

                            for i in 0..n / 2
                            {
                                b[ 0 ] = f_buf[ i * CHANNELS ];
                                b[ 1 ] = f_buf[ i * CHANNELS + 1 ];

                                let x = i16::from_le_bytes( b );

                                if do_alsa
                                {
                                    a_buf[ i ] = x;
                                }

                                s_buf.push_back( x );
                            }

                            if do_alsa
                            {
                                let mut discon = Vec::<String>::new();

                                for ( dev, ao ) in alsa_open_devices.iter_mut()
                                {
                                    let state = ao.pcm.state();

                                    let buffering =
                                        match state
                                        {
                                            alsa::pcm::State::Disconnected =>
                                            {
                                                log::warn!( "Alsa device disconnected. [{}]", dev );
                                                discon.push( String::from( dev ) );
                                                false
                                            }
                                        ,   alsa::pcm::State::XRun =>
                                            {
                                                log::debug!( "state [{:?}]", state );

                                                if let Ok( ( avail, delay ) ) = ao.pcm.avail_delay()
                                                {
                                                    log::debug!( "state [{:?}] avail:{} delay:{}", state, avail, delay );
                                                }

                                                let _ = ao.pcm.prepare();

                                                ao.buf.clear();
                                                ao.buf_flag = true;
                                                true
                                            }
                                        ,   alsa::pcm::State::Prepared =>
                                            {
                                                log::debug!( "state [{:?}]", state );
                                                let _ = ao.pcm.start();
                                                true
                                            }

                                        ,   alsa::pcm::State::Running =>
                                            {
                                                if ao.buf_flag == true && ao.buf.len() < ALSA_INIT_BUFFERING_SIZE
                                                {
                                                    true
                                                }
                                                else
                                                {
                                                    let io = ao.pcm.io_i16().unwrap();

                                                    if ao.buf.len() != 0
                                                    {
                                                        log::debug!( "Alsa init buffring initbuf:{} + read:{} = {}", ao.buf.len(), n / 2, ao.buf.len() + n / 2 );

                                                        let slices = ao.buf.as_slices();

                                                        for slice in vec![ slices.0, slices.1 ]
                                                        {
                                                            let samples = slice.len();

                                                            if samples != 0
                                                            {
                                                                match io.writei( slice )
                                                                {
                                                                    Ok( x ) if x != samples / CHANNELS =>
                                                                    {
                                                                        log::warn!( "Alsa device write bytes unmatch. write:{} src:{}", x, n/2 );
                                                                    }
                                                                ,   Ok( _ ) => {}
                                                                ,   Err( x ) =>
                                                                    {
                                                                        log::warn!( "Alsa device write error. [{}]", x );
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        ao.buf.clear();
                                                        ao.buf_flag = false;
                                                    }

                                                    let samples = n / 2;

                                                    match io.writei( &a_buf[ 0..samples ] )
                                                    {
                                                        Ok( x ) if x != samples / CHANNELS =>
                                                        {
                                                            log::warn!( "Alsa device write bytes unmatch. write:{} src:{}", x, n/2 );
                                                        }
                                                    ,   Ok( _ ) => {}
                                                    ,   Err( x ) =>
                                                        {
                                                            log::warn!( "Alsa device write error. [{}]", x );
                                                        }
                                                    }

                                                    false
                                                }
                                            }
                                        ,   _ =>
                                            {
                                                log::debug!( "state [{:?}]", state );
                                                true
                                            }
                                        };

                                    if buffering
                                    {
                                        if ao.buf.len() > F_BUF_SIZE / 2
                                        {
                                            let len = ao.buf.len() - F_BUF_SIZE / 2;

                                            ao.buf.drain( 0..len );
                                        }

                                        let samples = n / 2;

                                        for i in 0..samples
                                        {
                                            ao.buf.push_back( a_buf[ i ] );
                                        }
                                    }
                                }

                                for dev in discon
                                {
                                    let _ = alsa_open_devices.remove( &dev );
                                }
                            }
                        }

                        if s_buf.len() > fft_buf_size * CHANNELS + s_buf_delay_size
                        {
                            if s_buf.len() > fft_buf_size * CHANNELS + s_buf_delay_size
                            {
                                let len = s_buf.len() - fft_buf_size * CHANNELS + s_buf_delay_size;

                                s_buf.drain( 0..len );
                            }

                            {
                                let mut s_buf_iter = s_buf.iter();

                                let mut sum_l  : f32 = 0.0;
                                let mut sum_r  : f32 = 0.0;

                                let mut peak_l : f32 = 0.0;
                                let mut peak_r : f32 = 0.0;

                                for i in 0..fft_buf_size
                                {
                                    let l = *s_buf_iter.next().unwrap() as f32 / std::i16::MAX as f32;
                                    let r = *s_buf_iter.next().unwrap() as f32 / std::i16::MAX as f32;

                                    fft_i_l[ i ] = Complex::< f32 >::new( l , 0.0 );
                                    fft_i_r[ i ] = Complex::< f32 >::new( r , 0.0 );

                                    sum_l += l * l * 10000.0;
                                    sum_r += r * r * 10000.0;

                                    peak_l = peak_l.max( l * l * 10000.0 );
                                    peak_r = peak_r.max( r * r * 10000.0 );
                                }

                                spd.rms_l  = ( ( sum_l / fft_buf_size as f32 ).sqrt().log10() * 20.0 * CORRECTION_3 ).min( 1000.0 ).max( 0.0 ) as u32;
                                spd.rms_r  = ( ( sum_r / fft_buf_size as f32 ).sqrt().log10() * 20.0 * CORRECTION_3 ).min( 1000.0 ).max( 0.0 ) as u32;
                                spd.peak_l = ( peak_l.sqrt().log10() * 20.0 * CORRECTION_3 ).min( 1000.0 ).max( 0.0 ) as u32;
                                spd.peak_r = ( peak_r.sqrt().log10() * 20.0 * CORRECTION_3 ).min( 1000.0 ).max( 0.0 ) as u32;
                            }

                            s_buf.drain( 0..fft_buf_slide_size * CHANNELS );

                            let fft_o_l = fft_engine_chfft.forward( fft_i_l.as_slice() );
                            let fft_o_r = fft_engine_chfft.forward( fft_i_r.as_slice() );

                            for p in 0..spec_len
                            {
                                spec_amp_l[ p ] = 0.0;
                                spec_amp_r[ p ] = 0.0;
                            }

                            for i in 0..fft_spec_size
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

