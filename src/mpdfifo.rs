use std::io::{ self, Read };
use std::collections::VecDeque;
use std::fs::File;

use std::os::unix::io::{ AsRawFd };
use libc::{F_GETFL, F_SETFL, fcntl, O_NONBLOCK};

use tokio::time::{ timeout, Duration, Instant };
use tokio::sync::{ oneshot, mpsc };
use tokio::prelude::*;

use chfft::CFft1D;
use num_complex::Complex;

pub struct BarData
{
  pub bar_l : Vec::< f32 >
, pub bar_r : Vec::< f32 >
, pub bar_h : Vec::< u32 >
, pub sbuf  : usize
, pub fcnt  : usize
, pub rcnt  : usize
, pub scnt  : usize
, pub delay : Duration
}

impl BarData
{
    pub fn new() -> BarData
    {
        BarData
        {
          bar_l : Vec::< f32 >::new()
        , bar_r : Vec::< f32 >::new()
        , bar_h : Vec::< u32 >::new()
        , sbuf  : 0
        , fcnt  : 0
        , rcnt  : 0
        , scnt  : 0
        , delay : Duration::from_millis( 500 )
        }
    }
}

pub enum MpdFifoRequestType
{
    Nop
,   Shutdown
}

///
pub struct MpdFifoRequest
{
    pub req  : MpdFifoRequestType
,   pub tx   : oneshot::Sender< MpdFifoResult >
}

pub struct MpdFifoResult
{
}

///
impl MpdFifoRequest
{
    pub fn new() -> ( MpdFifoRequest, oneshot::Receiver< MpdFifoResult > )
    {
        let ( tx, rx ) = oneshot::channel::< MpdFifoResult >();

        (
            MpdFifoRequest{
                req         : MpdFifoRequestType::Nop
            ,   tx
            }
        ,   rx
        )
    }
}
