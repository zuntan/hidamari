//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::sync::Mutex;
use std::thread;

use actix_web::web;

use tokio::time::{ Duration };

use crate::event;
use crate::wssession;

const THREAD_SLEEP : Duration = Duration::from_millis( 65 );

pub async fn spectrum_responce_task(
    ctx     : web::Data< Mutex< super::Context > >
,   mut rx  : event::EventReceiver
)
-> Result< (), Box< dyn std::error::Error> >
{
    let sleep_dur = THREAD_SLEEP - event::EVENT_SHUTDOWN_TIMEOUT;

    let mut last_data = String::new();

    loop
    {
        thread::sleep( sleep_dur );

        if event::event_shutdown( &mut rx ).await
        {
            break;
        }

        let ctx = &mut ctx.lock().unwrap();

        if last_data != ctx.spec_data_json
        {
            last_data = String::from( &ctx.spec_data_json );

            for ( k, v ) in ctx.status_ws_sessions.iter()
            {
                if let wssession::WsSwssionType::Default = k.wst
                {
                    let _ = v.do_send( wssession::WsSessionMessage( String::from( &last_data ) ) );
                }
            }
        }
    }

    Ok(())
}
