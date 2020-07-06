//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::sync::Mutex;

use actix_web::web;

use tokio::time::{ delay_for, Duration };

use crate::event;
use crate::wssession;

const SP_TASK_SLEEP : Duration = Duration::from_millis( 63 );  // = 1/16 sec

pub async fn ws_responce_task(
    ctx     : web::Data< Mutex< super::Context > >
,   mut rx  : event::EventReceiver
)
-> Result< (), Box< dyn std::error::Error> >
{
    let sleep_dur = SP_TASK_SLEEP - event::EVENT_WAIT_TIMEOUT;

    let mut last_spec_data_json = String::new();
    let mut last_mpd_status_json = String::new();

    let enalbe_spec =
    {
        let ctx = &ctx.lock().unwrap();
        ctx.config.mpd_fifo != ""
    };

    log::info!( "ws_responce_task spec_data:{}", if enalbe_spec { "enable" } else { "disable" } );

    loop
    {
        delay_for( sleep_dur ).await;

        if event::event_shutdown( &mut rx ).await
        {
            break;
        }

        let ctx = &ctx.lock().unwrap();

        let sdj =
            if last_spec_data_json != ctx.spec_data_json
            {
                last_spec_data_json = String::from( &ctx.spec_data_json );
                enalbe_spec
            }
            else
            {
                false
            };

        let msj =
            if last_mpd_status_json != ctx.spec_data_json
            {
                last_mpd_status_json = String::from( &ctx.mpd_status_json );
                true
            }
            else
            {
                false
            };

        if sdj || msj
        {
            for ( k, v ) in ctx.ws_sessions.iter()
            {
                if let wssession::WsSwssionType::Default = k.wst
                {
                    if sdj
                    {
                        let _ = v.do_send( wssession::WsSessionMessage( String::from( &last_spec_data_json ) ) );
                    }

                    if msj
                    {
                        let _ = v.do_send( wssession::WsSessionMessage( String::from( &last_mpd_status_json ) ) );
                    }
                }
            }
        }
    }

    Ok(())
}
