//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io;
use std::pin::Pin;
use std::boxed::Box;
use std::task::Poll;
use std::task::Context;
use std::ptr;
use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{ AtomicUsize, Ordering };

use tokio::fs::File;
use tokio::io::AsyncRead;
use tokio::time::{ delay_for, Duration, Instant };
use tokio::task;

use alsa::{ Direction, ValueOr };
use alsa::pcm::{ PCM, HwParams, Format, Access /*, State */ };

use lame_sys;

use std::os::raw::c_int;

///
#[derive(Debug)]
pub struct FileRangeRead
{
    pub file : Pin< Box< File > >
,   pub len  : u64
,   pub cur  : u64
}

///
impl FileRangeRead
{
    pub async fn new( mut file : File, start : u64, end : u64 ) -> io::Result< Self >
    {
        if let io::Result::Err( x ) = file.seek( std::io::SeekFrom::Start( start ) ).await
        {
            return io::Result::Err( x );
        }

        return io::Result::Ok(
            Self
            {
                file    : Box::pin( file )
            ,   len     : end - start
            ,   cur     : 0
            }
        )
    }

    pub fn len( &self ) -> u64
    {
        self.len
    }
}

///
impl AsyncRead for FileRangeRead
{
    fn poll_read( mut self : Pin< &mut Self >, cx : &mut Context<'_> , dst: &mut [u8] )
        -> Poll< std::io::Result< usize > >
    {
        if self.cur >= self.len
        {
            Poll::Ready( Ok ( 0 ) )
        }
        else
        {
            match self.file.as_mut().poll_read( cx, dst )
            {
                Poll::Pending =>
                {
                    Poll::Pending
                }
            ,   Poll::Ready( x ) =>
                {
                    match x
                    {
                        Err( e ) =>
                        {
                            Poll::Ready( Err( e ) )
                        }
                    ,   Ok( n ) =>
                        {
                            let mut n = n as u64;

                            if self.cur + n >= self.len
                            {
                                n = self.len - self.cur;
                            }

                            self.cur += n;
                            Poll::Ready( Ok ( n as usize ) )
                        }
                    }
                }
            }
        }
    }
}

///
#[derive(Debug)]
pub struct AlsaCaptureLameEncodeParam
{
    pub a_rate      : Option<u32>
,   pub a_channels  : Option<u8>
,   pub a_buffer_t  : Option<u32>
,   pub a_period_t  : Option<u32>
,   pub lm_brate    : Option<u32>
,   pub lm_a_brate  : Option<u32>
}

unsafe fn alloc< T >( len: usize ) -> *mut T
{
    let mut vec = Vec::< T >::with_capacity( len );
    vec.set_len( len );
    Box::into_raw( vec.into_boxed_slice() ) as *mut T
}

unsafe fn free< T >( raw: *mut T, len : usize )
{
    let s = std::slice::from_raw_parts_mut( raw, len );
    let _ = Box::from_raw( s );
}

unsafe fn slice_mut<'a, T>( raw: *mut T, len : usize ) -> &'a mut [T]
{
    std::slice::from_raw_parts_mut( raw, len )
}

unsafe fn slice<'a, T>( raw: *mut T, len : usize ) -> &'a [T]
{
    std::slice::from_raw_parts( raw, len )
}

static ALSA_CAPTURE_LAME_ENCODE_COUNTER : AtomicUsize = AtomicUsize::new( 0 );

///
pub struct AlsaCaptureLameEncode
{
    id      : usize
,   dev     : String
,   pcm     : PCM
,   gfp     : lame_sys::lame_t
,   buf     : VecDeque< u8 >
,   abuf_len: usize
,   abuf    : *mut i16
,   lbuf_len: usize
,   lbuf    : *mut u8
}

impl fmt::Debug for AlsaCaptureLameEncode
{
    fn fmt( &self, f: &mut fmt::Formatter<'_> ) -> fmt::Result
    {
        write!( f, "AlsaCaptureLameEncode:[{:?}] dev:[{}]", &self.id, &self.dev )
    }
}

impl AlsaCaptureLameEncode
{
    pub fn new( dev : String, param : AlsaCaptureLameEncodeParam ) -> io::Result< Self >
    {
        log::debug!( "AlsaCaptureLameEncode::new [{}]", dev );

        match PCM::new( &dev, Direction::Capture, true )
        {
            Ok( pcm ) =>
            {
                let ( rate, ch ) =
                {
                    let hwp = HwParams::any( &pcm ).unwrap();

                    if let Some( x ) = param.a_rate
                    {
                        if let Err( x ) = hwp.set_rate( x, ValueOr::Nearest )
                        {
                            log::error!( "AlsaCaptureLameEncode hwp.set_rate error. {:?}", x );
                        }
                    }

                    if let Some( x ) = param.a_channels
                    {
                        if let Err( x ) = hwp.set_channels( x as u32 )
                        {
                            log::error!( "AlsaCaptureLameEncode hwp.set_channels error. {:?}", x );
                        }
                    }

                    if let Err( x ) = hwp.set_format( Format::s16() )
                    {
                        log::error!( "AlsaCaptureLameEncode hwp.set_format error. {:?}", x );
                    }

                    if let Some( x ) = param.a_buffer_t
                    {
                        if let Err( x ) = hwp.set_buffer_time_near( x, ValueOr::Nearest )
                        {
                            log::error!( "AlsaCaptureLameEncode hwp.set_buffer_time_near error. {:?}", x );
                        }
                    }

                    if let Some( x ) = param.a_period_t
                    {
                        if let Err( x ) = hwp.set_period_time_near( x, ValueOr::Nearest )
                        {
                            log::error!( "AlsaCaptureLameEncode hwp.set_period_time_near error. {:?}", x );
                        }
                    }

                    if let Err( x ) = hwp.set_access( Access::RWInterleaved )
                    {
                        log::error!( "AlsaCaptureLameEncode hwp.set_access error. {:?}", x );
                    }

                    if let Err( x ) = pcm.hw_params( &hwp )
                    {
                        log::error!( "AlsaCaptureLameEncode hw_params error. {:?}", x );

                        return Err( io::Error::new( io::ErrorKind::ConnectionRefused, x ) );
                    }

                    (
                        hwp.get_rate().unwrap()
                    ,   hwp.get_channels().unwrap() as u8
                    )
                };

                if let Err( x ) = pcm.start()
                {
                    log::error!( "AlsaCaptureLameEncode start error. {:?}", x );

                    Err( io::Error::new( io::ErrorKind::ConnectionRefused, x ) )
                }
                else
                {
                    let gfp =
                    unsafe
                    {
                        lame_sys::lame_init()
                    };

                    if gfp == ptr::null_mut()
                    {
                        log::error!( "lame_init error. " );

                        Err( io::Error::new( io::ErrorKind::Other, "lame_init error" ) )
                    }
                    else
                    {
                        unsafe
                        {
                            lame_sys::lame_set_in_samplerate( gfp, rate as c_int );
                            lame_sys::lame_set_out_samplerate( gfp, rate as c_int );
                            lame_sys::lame_set_num_channels( gfp, ch as i32 );
                            lame_sys::lame_set_mode( gfp, if ch == 2 { lame_sys::MPEG_mode::STEREO } else { lame_sys::MPEG_mode::MONO } );

                            let mut s = false;

                            if !s
                            {
                                if let Some( x ) = param.lm_brate
                                {
                                    if x > 0
                                    {
                                        lame_sys::lame_set_brate( gfp, x as c_int );
                                        lame_sys::lame_set_quality( gfp, 2 );
                                        s = true;
                                    }
                                }
                            }

                            if !s
                            {
                                if let Some( x ) = param.lm_a_brate
                                {
                                    if x > 0
                                    {
                                        lame_sys::lame_set_VBR( gfp, lame_sys::vbr_mode::vbr_abr );
                                        lame_sys::lame_set_VBR_mean_bitrate_kbps( gfp, x as c_int );
                                    }
                                }
                            }

                            if !s
                            {
                                lame_sys::lame_set_brate( gfp, 0 );
                                lame_sys::lame_set_VBR( gfp, lame_sys::vbr_default );
                                lame_sys::lame_set_VBR_quality( gfp, 2.0 );
                            }

                            lame_sys::lame_init_params( gfp );
                        };

                        let abuf_len    = ( ( rate as f32 / 10.0 ) * ch as f32 ) as usize;
                        let lbuf_len    = ( abuf_len as f32 * 1.25 + 7200.0 ) as usize;

                        Ok(
                            AlsaCaptureLameEncode
                            {
                                id          : ALSA_CAPTURE_LAME_ENCODE_COUNTER.fetch_add( 1, Ordering::SeqCst )
                            ,   dev
                            ,   pcm
                            ,   gfp
                            ,   buf         : VecDeque::< u8 >::new()
                            ,   abuf_len
                            ,   abuf        : unsafe{ alloc::< i16 >( abuf_len ) }
                            ,   lbuf_len
                            ,   lbuf        : unsafe{ alloc::< u8 >( lbuf_len ) }
                            }
                        )
                    }
                }
            }
        ,   Err( x ) =>
            {
                log::error!( "AlsaCaptureLameEncode open error. {:?}", x );

                Err( io::Error::new( io::ErrorKind::NotFound, x ) )
            }
        }
    }
}

impl Drop for AlsaCaptureLameEncode
{
    fn drop( &mut self )
    {
        log::debug!( "AlsaCaptureLameEncode drop [{:?}]", &self );

        unsafe
        {
            lame_sys::lame_close( self.gfp );
            free( self.lbuf, self.lbuf_len );
            free( self.abuf, self.abuf_len );
        };
    }
}

unsafe impl Send for AlsaCaptureLameEncode {}

///
impl AsyncRead for AlsaCaptureLameEncode
{
    fn poll_read( mut self : Pin< &mut Self >, cx : &mut Context<'_> , dst: &mut [u8] )
        -> Poll< std::io::Result< usize > >
    {
        log::debug!( "AlsaCaptureLameEncode::poll_read [{:?}] state:[{:?}]", &self, self.pcm.state() );

        if self.pcm.state() == alsa::pcm::State::Disconnected
        {
            return Poll::Ready( Ok( 0 ) );
        }

        if self.buf.is_empty()
        {
            match
            {
                let abuf = unsafe{ slice_mut( self.abuf, self.abuf_len ) };
                let io = self.pcm.io_i16().unwrap();
                io.readi( abuf )
            }
            {
                Ok( alen ) =>
                {
                    let llen =
                    unsafe
                    {
                        lame_sys::lame_encode_buffer_interleaved(
                            self.gfp
                        ,   self.abuf
                        ,   alen as c_int
                        ,   self.lbuf
                        ,   self.lbuf_len as c_int
                        )
                    };

                    if llen > 0
                    {
                        let llen = llen as usize;

                        if llen < self.lbuf_len
                        {
                            let lbuf = unsafe{ slice( self.lbuf, llen ) };
                            self.buf.extend( lbuf );
                        }
                    }
                    else
                    {
                        log::debug!( "lame error alen:{:?} llen:{:?}", alen, llen );
                    }
                }
            ,   Err( x ) =>
                {
                    log::debug!( "alsa error {:?}", x );
                }
            }
        }

        if self.buf.is_empty()
        {
            log::debug!( "Poll::Pending" );

            let waker = cx.waker().clone();

            task::spawn(
                async
                {
                    delay_for( Duration::from_millis( 100 ) ).await;
                    waker.wake()
                }
            );

            Poll::Pending
        }
        else
        {
            let     dl = dst.len();
            let mut dp = 0;

            while dp < dl && !self.buf.is_empty()
            {
                dst[ dp ] = self.buf.pop_front().unwrap();
                dp += 1;
            }

            Poll::Ready( Ok( dp ) )
        }
    }
}
