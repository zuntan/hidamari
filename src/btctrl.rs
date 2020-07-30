//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::io;

use tokio::time::{ delay_for, Duration, Instant };
use tokio::sync::{ mpsc };

use serde::{ Serialize, /* Deserialize */ };

use crate::context;
use crate::event;
use crate::bt;

pub async fn btctrl_task(
    arwlctx : context::ARWLContext
,   mut rx  : mpsc::Receiver< event::EventRequest >
)
-> io::Result< ()  >
{
    log::debug!( "btctrl start." );

    loop
    {
        if event::event_shutdown( &mut rx ).await
        {
            break;
        }


    }

    log::debug!( "btctrl stop." );

    Ok(())
}

