$(
function()
{
	var hidamari =
	{
		websocket : function()
		{
			return(
			{
				ws_status : null
			,	ws_spec_l : null
			,	ws_spec_r : null
			,	ws_rms_l  : null
			,	ws_rms_r  : null
			,	ws_spec_t : null
			,	ws_spec_h : null

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

			,	open : function()
				{
					this.update();

					var that = this;

					var ws_proto = location.protocol == 'https:' ? 'wss:' : 'ws:';
				    var ws = new WebSocket( ws_proto + '//' + location.host + '/ws' );

					var ws_reopen = function()
					{
						this.ws_status = null;
						this.ws_spec_l = null;
						this.ws_spec_r = null;
						this.ws_rms_l  = null;
						this.ws_rms_r  = null;
						this.ws_spec_t = null;
						setTimeout( this.open, 1000 );
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

						var t = [];
						if( kv[ 'Track' ] ) { t.push( kv[ 'Track' ] ); }
						if( kv[ 'Album' ] ) { t.push( kv[ 'Album' ] ); }
						if( kv[ 'Artist' ] ) { t.push( kv[ 'Artist' ] ); }

						kv[ '_title_2' ] = t.join( " : " );

						var d = "";

						d = kv[ 'Time' ] ? kv[ 'Time' ] : "";
						//	d = kv[ 'duration' ] ? kv[ 'duration' ] : "";

						if( d != "" )
						{
							d = this.format_time( d );
						}

						kv[ '_time' ] = d;

						kv[ '_pos' ] = parseInt( kv[ 'Pos' ] );
						kv[ '_id'  ] = kv[ 'Id' ]
					}

					kv[ '_name' ] = v;


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
