$(
function()
{
	var hidamari =
	{
		websocket : function()
		{
			return(
			{
				ws_status 		: null
			,	ws_spec_l 		: null
			,	ws_spec_r	 	: null
			,	ws_rms_l 	 	: null
			,	ws_rms_r  		: null
			,	ws_spec_t		: null
			,	ws_spec_h		: null
			,	ws_bt_status	: null

			,	update : function( func )
				{
					if( arguments.length > 0 )
					{
						if( !this._cb_update ) { this._cb_update = Array(); }
						this._cb_update.push( func );
					}
					else
					{
						if( this._cb_update ) { var that = this; $.each( this._cb_update, function( i, o ) { o.call( that ) } ) }
					}
				}

			,	status_update : function( func )
				{
					if( arguments.length > 0 )
					{
						if( !this._cb_status_update ) { this._cb_status_update = Array(); }
						this._cb_status_update.push( func );
					}
					else
					{
						if( this._cb_status_update ) { var that = this; $.each( this._cb_status_update, function( i, o ) { o.call( that ) } ) }
					}
				}

			,	spec_update : function( func )
				{
					if( arguments.length > 0 )
					{
						if( !this._cb_spec_update ) { this._cb_spec_update = Array(); }
						this._cb_spec_update.push( func );
					}
					else
					{
						if( this._cb_spec_update ) { var that = this; $.each( this._cb_spec_update, function( i, o ) { o.call( that ) } ) }
					}
				}

			,	bt_status_update : function( func )
				{
					if( arguments.length > 0 )
					{
						if( !this._cb_bt_status_update ) { this._cb_bt_status_update = Array(); }
						this._cb_bt_status_update.push( func );
					}
					else
					{
						if( this._cb_bt_status_update ) { var that = this; $.each( this._cb_bt_status_update, function( i, o ) { o.call( that ) } ) }
					}
				}

			,	open : function()
				{
					this.update();

					var that = this;

					var ws_proto = location.protocol == 'https:' ? 'wss:' : 'ws:';
				    var ws = new WebSocket( ws_proto + '//' + location.host + '/ws' );

					var ws_reopen = function()
					{
						that.ws_status = null;
						that.ws_spec_l = null;
						that.ws_spec_r = null;
						that.ws_rms_l  = null;
						that.ws_rms_r  = null;
						that.ws_spec_t = null;
						that.ws_bt_status = null;
						setTimeout( function() { that.open(); }, 1000 );
					};

					ws.onclose = ws_reopen;
				    ws.onError = ws_reopen;
				    ws.onmessage = function( e )
			        {
						j_data = $.parseJSON( e.data );

						if( j_data && j_data.Ok )
						{
							if( j_data.Ok.status )
							{
								that.ws_status = j_data.Ok.status
								that.status_update();
							}

							if( j_data.Ok.spec_t )
							{
								that.ws_spec_l = j_data.Ok.spec_l
								that.ws_spec_r = j_data.Ok.spec_r
								that.ws_spec_t = j_data.Ok.spec_t
								that.ws_rms_l  = j_data.Ok.rms_l
								that.ws_rms_r  = j_data.Ok.rms_r
							}

							if( j_data.Ok.spec_h )
							{
								that.ws_spec_h = j_data.Ok.spec_h
							}

							if( j_data.Ok.spec_t || j_data.Ok.spec_h )
							{
								that.spec_update();
							}

							if( j_data.Ok.bt_status )
							{
								that.ws_bt_status = j_data.Ok.bt_status
								that.bt_status_update();
							}

							if( j_data.Ok.bt_notice )
							{
								var m = $( "#x_bt_notice" );


								if( m.length )
								{
									var n = j_data.Ok.bt_notice;

									if( n.cancel )
									{
										m.modal( 'hide' );
									}
									else
									{
										$( ".x_bt_notice_title", m ).text( n.title );

										var msg = "";

										if( n.passkey && n.passkey != "" )
										{
											msg += "passkey [" + n.passkey + "] ";
										}

										if( n.entered && n.entered != "" )
										{
											msg += "entered [" + n.entered + "] ";
										}

										$( ".x_bt_notice_msg", m ).text( msg );

										var device = "";

										if( n.device )
										{
											device += "Device : " + n.device.alias + " [" + n.device.address + "] ";

											if( n.device.audio_source )
											{
												device += " (audio source) ";
											}

											if( n.device.audio_sink )
											{
												device += " (audio sink) ";
											}
										}

										$( ".x_bt_notice_dev", m ).text( device );

										$( ".x_bt_notice_cancel, .x_bt_notice_apply, .x_bt_notice_close", m ).data( "reply_token", n.reply_token );
										$( ".x_bt_notice_cancel, .x_bt_notice_apply"	, m	).toggle( n.reply_token != "" );
										$( ".x_bt_notice_close"  						, m ).toggle( n.reply_token == "" );

										$( ".x_bt_notice_cancel", m ).one( 'click',
											function()
											{
												var reply_token = $(this).data( "reply_token" );

												if( reply_token != "" )
												{
													$.getJSON( "/bt_reply", { reply_token : reply_token , ok : false } );
												}
											}
										);

										$( ".x_bt_notice_apply", m ).one( 'click',
											function()
											{
												var reply_token = $(this).data( "reply_token" );

												if( reply_token != "" )
												{
													$.getJSON( "/bt_reply", { reply_token : reply_token , ok : true } );
												}
											}
										);

										m.modal( 'show' );
									}
								}
							}

							that.update();
						}
					}
				}
			} );
		}

	,	format_time : function( d )
		{
			d = parseInt( d );

			var s = d % 60;
			var m = ( ( d - s ) / 60 ) % 60;
			var h = ( ( d - s - m * 60 ) / 60 * 60 ) % 60;

			return 	( h != 0 ) ? ( '00' + h ).slice( -2 ) + ":" : ""
					+ ( '00' + m ).slice( -2 ) + ":"
					+ ( '00' + s ).slice( -2 )
					;
		}

	,	parse_flds : function( flds )
		{
			var kv = {};

			for( var i = 0 ; i < flds.length ; ++i )
			{
				kv[ flds[ i ][0] ] = flds[ i ][1];
			}

			return kv;
		}

	,	parse_list : function( flds )
		{
			var t_flds = flds.slice( 0, flds.length );

			t_flds.reverse();

			var list = [];
			var kv = {};

			for( var i = 0 ; i < t_flds.length ; ++i )
			{
				var k = t_flds[ i ][0];
				var v = t_flds[ i ][1];

				if( k == 'directory' || k == 'file' )
				{
					var n = v.split("/").pop();

					if( k == 'file' )
					{
						kv[ '_title_1' ] = n;

						if( kv[ 'Title' ] && kv[ 'Title' ] != "" )
						{
							kv[ '_title_1' ] = kv[ 'Title' ];
						}

						if( kv[ 'Name' ] && kv[ 'Name' ] != "" )
						{
							kv[ '_title_1' ] = kv[ 'Name' ];
						}

						var t = [];
						if( kv[ 'Track' ] ) { t.push( kv[ 'Track' ] ); }
						if( kv[ 'Album' ] ) { t.push( kv[ 'Album' ] ); }
						if( kv[ 'Artist' ] ) { t.push( kv[ 'Artist' ] ); }

						if( v.includes( "://" ) ){ t.push( v ); }

						kv[ '_title_2' ] = t.join( " : " );

						var d = "";

						d = kv[ 'Time' ] ? kv[ 'Time' ] : "";
						//	d = kv[ 'duration' ] ? kv[ 'duration' ] : "";

						if( d != "" )
						{
							d = this.format_time( d );
						}

						kv[ '_time' ] = d;

						kv[ '_pos'   ] = parseInt( kv[ 'Pos' ] );
						kv[ '_id'    ] = kv[ 'Id' ];
						kv[ '_name'  ] = v;
					}

					kv[ '_path' ] = v;


					list.push( [ k, n, kv ] );
					kv = {};
				}
				else
				{
					kv[ k ] = v;
				}
			}

			if( Object.keys( kv ).length )
			{
				list.push( [ 'info', '', kv ] );
			}

			list.reverse();

			return list;
		}

	,	flush_item : function()
		{
			var flush_item_impl = function( _f, _a )
			{
				return function()
				{
					$.each( _a
					,	function( i, v )
						{
							$(v).toggleClass( "x_flush", _f );
						}
					);
				};
			}

			for( var i = 0 ; i < 4 ; ++i )
			{
				setTimeout(
					flush_item_impl( i % 2, arguments )
				,	i * 75
				);
			}
		}

	,	flush_and_hide_item : function()
		{
			var flush_item_impl = function( _f, _a )
			{
				return function()
				{
					$.each( _a
					,	function( i, v )
						{
							$(v).toggleClass( "x_flush", _f );
						}
					);
				};
			}

			var hide_item_impl = function( _a )
			{
				return function()
				{
					$.each( _a
					,	function( i, v )
						{
							$(v).hide( 'fast' )
						}
					);
				};
			}

			var i = 0;

			for( ; i < 4 ; ++i )
			{
				setTimeout(
					flush_item_impl( i % 2, arguments )
				,	i * 75
				);
			}

			setTimeout(
				hide_item_impl( arguments )
			,	( 10 ) * 75
			);
		}

	,	select_item : function( /* bool, array */ )
		{
			var _a = Array.from( arguments );
			var _f = _a.shift();

			$.each( _a
			,	function( i, v )
				{
					$(v).toggleClass( "x_select", _f );
				}
			);
		}

	,	ajax_setup : function()
		{
			var ajax_state_err = $( "div.x_ajax_state_err" );
			ajax_state_err.hide();

			var ajax_state = $( "div.x_ajax_state" );
			ajax_state.hide();

			var ajax_state_t = null;

			$( document ).ajaxStart(
				function() {
					ajax_state_err.hide();
					ajax_state_t = setTimeout( function() { ajax_state.show(); }, 125 );
				}
			);

			$( document ).ajaxStop(
				function() {
					if( ajax_state_t )
					{
						clearTimeout( ajax_state_t );
						ajax_state_t = null;
					}
					ajax_state.hide();
				}
			);

			$( document ).ajaxError(
				function() {
					ajax_state_err.show();
				}
			);
		}
	}

	$.extend( { hidamari : $.extend( hidamari, $.hidamari || {} ) } );
}
);
