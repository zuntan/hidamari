//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use tokio::time::{ timeout, Duration };
use tokio::sync::{ oneshot, mpsc };

#[derive(Debug)]
pub enum EventRequestType
{
    Nop
,   Shutdown
}

///
pub struct EventRequest
{
    pub req  : EventRequestType
,   pub tx   : oneshot::Sender< EventResult >
}

pub struct EventResult
{
}

pub type EventSender            = mpsc::Sender< EventRequest >;
pub type EventReceiver          = mpsc::Receiver< EventRequest >;
pub type EventResultReceiver    = oneshot::Receiver< EventResult >;

pub fn make_channel()
    -> ( EventSender, EventReceiver )
{
    let ( tx, rx ) = mpsc::channel::< EventRequest >( 4 );
    ( tx, rx )
}

pub fn new_request()
    -> ( EventRequest, EventResultReceiver )
{
    let ( tx, rx ) = oneshot::channel::< EventResult >();

    (
        EventRequest{
            req         : EventRequestType::Nop
        ,   tx
        }
    ,   rx
    )
}

pub const EVENT_WAIT_TIMEOUT : Duration = Duration::from_millis( 5 );

pub async fn event( rx : &mut EventReceiver ) -> Option< EventRequest >
{
    match timeout( EVENT_WAIT_TIMEOUT, rx.recv() ).await
    {
        Ok( recv ) =>
        {
            let recv = recv.unwrap();

            log::debug!( "recv [{:?}]", recv.req );

            return Some( recv );
        }
    ,   Err(_) => {}
    }

    None
}

pub async fn event_shutdown( rx : &mut EventReceiver ) -> bool
{
    if let Some( recv ) = event( rx ).await
    {
        recv.tx.send( EventResult{} ).ok();
        true
    }
    else
    {
        false
    }
}
