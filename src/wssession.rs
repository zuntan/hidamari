//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

use std::cmp::Eq;
use std::hash::{ Hash, Hasher };
use std::ptr;
use std::sync::{ Arc, Mutex };
use std::ops::{ Deref, DerefMut };
use std::collections::hash_map::{ HashMap };

use actix_web::{ web };
use actix::{ Actor, Recipient, ActorContext, AsyncContext, StreamHandler, Message, Handler };
use actix_web_actors::ws;

pub type WsSessions = HashMap< Arc< WsSession >, Recipient< WsSessionMessage > >;

pub enum WsSwssionType
{
    Default
,   Special
}

pub struct WsSession
{
        ctx : web::Data< Mutex< super::Context > >
,       wsn : u64
, pub   wst : WsSwssionType
}

impl PartialEq for WsSession
{
    fn eq( &self, other: &Self ) -> bool
    {
        ptr::eq( self, other )
    }
}

impl Eq for WsSession {}

impl Hash for WsSession
{
    fn hash< H: Hasher >( &self, state: &mut H )
    {
        ptr::hash( self, state );
    }
}

pub struct ArcWsSession( Arc< WsSession > );

impl Deref for ArcWsSession
{
    type Target = Arc< WsSession >;

    fn deref( &self ) -> &Self::Target
    {
        &self.0
    }
}

impl DerefMut for ArcWsSession
{
    fn deref_mut( &mut self ) -> &mut Self::Target
    {
        &mut self.0
    }
}

impl Into< ArcWsSession > for Arc< WsSession >
{
    fn into( self ) -> ArcWsSession {
        ArcWsSession( self )
    }
}

impl ArcWsSession
{
    pub fn new( ctx : & web::Data< Mutex< super::Context > > ) -> ArcWsSession
    {
        Self::with_type( ctx, WsSwssionType::Default )
    }

    pub fn with_type( ctx : & web::Data< Mutex< super::Context > >, wst : WsSwssionType ) -> ArcWsSession
    {
        let wsn =
        {
            let mut ctx = ctx.lock().unwrap();
            ctx.ws_sess_no += 1;
            ctx.ws_sess_no
        };

        Arc::new( WsSession{ ctx : ctx.clone(), wsn, wst } ).into()
    }
}

impl Actor for ArcWsSession
{
    type Context = ws::WebsocketContext<Self>;

    fn started( &mut self, wsctx: &mut Self::Context )
    {
        let sz =
        {
            let t : &Arc< WsSession > = self.deref();

            let mut ctx = self.ctx.lock().unwrap();
            ctx.ws_sessions.insert( t.clone(), wsctx.address().recipient() );
            ctx.ws_sessions.len()
        };

        log::debug!( "start wsn:{} sz:{}", self.wsn, sz );
    }

    fn stopped( &mut self, _wsctx: &mut Self::Context )
    {
        let sz =
        {
            let t : &Arc< WsSession > = self.deref();

            let mut ctx = self.ctx.lock().unwrap();
            ctx.ws_sessions.remove( t );
            ctx.ws_sessions.len()
        };

        log::debug!( "stop wsn:{} sz:{}", self.wsn, sz );
    }
}

impl StreamHandler< Result< ws::Message, ws::ProtocolError > > for ArcWsSession
{
    fn handle(
        &mut self
    ,   msg: Result< ws::Message, ws::ProtocolError >
    ,   ctx: &mut Self::Context
    ) {
        match msg
        {
            Ok( x ) =>
            {
                match x
                {
                    ws::Message::Text( text ) =>
                    {
                        log::debug!( "text [{}]", text );
                    }

                ,   ws::Message::Ping( bytes ) =>
                    {
                        log::debug!( "ping {:?} bytes", bytes.len() );
                        ctx.pong( &bytes )
                    }

                ,   ws::Message::Close( reason ) =>
                    {
                        log::debug!( "close [{:?}]", &reason );
                        ctx.close( reason );
                        ctx.stop();
                    }
                ,   _ => {}
                }
            }
        ,   Err( ref x ) =>
            {
                log::debug!( "close [{:?}]", x );
                ctx.stop();
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsSessionMessage( pub String );

impl Handler< WsSessionMessage > for ArcWsSession
{
    type Result = ();

    fn handle( &mut self, msg: WsSessionMessage, ctx: &mut Self::Context )
    {
        ctx.text( msg.0 );
    }
}

