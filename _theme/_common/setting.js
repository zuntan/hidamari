$(
function()
{
	// common function

	var ws_status = null;
	var ws_spec_l = null;
	var ws_spec_r = null;
	var ws_rms_l  = null;
	var ws_rms_r  = null;
	var ws_spec_t = null;
	var ws_spec_h = null;

	var ws_open = function()
	{
		if( monitor_update )
		{
			monitor_update();
		}

		$.getJSON( "/spec_head" )
			.always( function()
				{
					ws_spec_h = null;
				}
			)
			.done( function( json )
				{
				    if( json.Ok && json.Ok.spec_h )
				    {
						ws_spec_h = json.Ok.spec_h
						monitor_update();
					}
				}
			);

		var ws_proto = location.protocol == 'https:' ? 'wss:' : 'ws:';
	    var ws = new WebSocket( ws_proto + '//' + location.host + '/ws' );

		var ws_reopen = function()
		{
			ws_status = null;
			ws_spec_l = null;
			ws_spec_r = null;
			ws_rms_l  = null;
			ws_rms_r  = null;
			ws_spec_t = null;
			setTimeout( ws_open, 1000 );
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
					ws_status = j_data.Ok.status
				}

				if( j_data.Ok.spec_t )
				{
					ws_spec_l = j_data.Ok.spec_l
					ws_spec_r = j_data.Ok.spec_r
					ws_spec_t = j_data.Ok.spec_t
					ws_rms_l  = j_data.Ok.rms_l
					ws_rms_r  = j_data.Ok.rms_r
				}

				monitor_update();
			}
		};
	};

	ws_open();

	var format_time = function( d )
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

	var parse_flds = function( flds )
	{
		var kv = {};

		for( var i = 0 ; i < flds.length ; ++i )
		{
			kv[ flds[ i ][0] ] = flds[ i ][1];
		}

		return kv;
	}

	var parse_list = function( flds )
	{
		var list = [];

		flds.reverse();

		var kv = {};

		for( var i = 0 ; i < flds.length ; ++i )
		{
			var k = flds[ i ][0];
			var v = flds[ i ][1];

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
						d = format_time( d );
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

	var flush_item = function()
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

	var select_item = function( /* bool, array */ )
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

	// init top

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

	// Config Load

	var update_theme = function( json )
	{
		if( json.themes )
		{
			var t = $( ".x_st_themes" );

			for( var i = 0 ; i < json.themes.length ; ++i )
			{
				var h = "";
				h += '<option value="' + json.themes[ i ]  + '" ';
				h += ( json.themes[ i ] == json.theme ? ' selected="selected" ' : '' );
				h += '>' + json.themes[ i ] + '</option>';

				t.append( $( h ) );
			}
		}
	}

	var update_bluetooth = function( json )
	{
		$( ".x_st_bluetooth" ).prop( 'checked', !!json.bluetooth );
	}

	var update_anidelay = function( json )
	{
		$( ".x_st_anidelay" ).val( json.spec_delay )
		$( ".x_st_anidelay_val" ).text( json.spec_delay )
	}

	$.getJSON( "/config" )
		.always( function()
			{
				$( ".x_st_themes > option" ).remove();
			}
		)
		.done( function( json )
			{
				update_theme( json )
				update_anidelay( json );
				update_bluetooth( json );
			}
		);

	$( ".x_st_anidelay" ).change(
		function()
		{
			$( ".x_st_anidelay_val" ).text( $(this).val() );
		}
	);

	$( ".x_st_theme_apply" ).click(
		function()
		{
			var theme = $( ".x_st_themes" ).val();

			$.getJSON( "/config", { update : JSON.stringify( { theme : theme } ) } )
				.always( function()
					{
						$( ".x_st_themes > option" ).remove();
					}
				)
				.done( function( json )
					{
						update_theme( json );
					}
				);
		}
	);

	$( ".x_st_anidelay_apply" ).click(
		function()
		{
			var spec_delay = parseInt( $( ".x_st_anidelay" ).val() );

			$.getJSON( "/config", { update : JSON.stringify( { spec_delay : spec_delay } ) } )
				.always( function()
					{
					}
				)
				.done( function( json )
					{
						update_anidelay( json );
					}
				);
		}
	);

	// Development Section

	$( ".x_st_dev_close" ).click(
		function()
		{
			$( ".x_st_dev_close" ).hide();
			$( ".x_st_dev_open" ).show();
			$( ".x_st_dev" ).hide( "normal" );
		}
	);

	$( ".x_st_dev_open" ).click(
		function()
		{
			$( ".x_st_dev_close" ).show();
			$( ".x_st_dev_open" ).hide();
			$( ".x_st_dev" ).show( "normal" );
		}
	);

	$( ".x_st_dev" ).hide();
	$( ".x_st_dev_close" ).click();

	$( ".x_st_dev_status_update" ).click(
		function()
		{
			var x = $( ".x_st_dev_status_result" );

			$.getJSON( "/status" )
				.always( function()
					{
						x.val( "err" );
					}
				)
				.done( function( json )
					{
						x.val( JSON.stringify( json ) );
					}
				);
		}
	);

	$( ".x_st_dev_cmd_ddi" ).click(
		function()
		{
			$( ".x_st_dev_cmd_cmd" ).val( $(this).text() );
		}
	);

	$( ".x_st_dev_cmd_exec" ).click(
		function()
		{
			var x = $( ".x_st_dev_cmd_result" );

			var cmd  = $( ".x_st_dev_cmd_cmd"  ).val();
			var arg1 = $( ".x_st_dev_cmd_arg1" ).val();
			var arg2 = $( ".x_st_dev_cmd_arg2" ).val();
			var arg3 = $( ".x_st_dev_cmd_arg3" ).val();

			$.getJSON( "/cmd", { cmd : cmd , arg1 : arg1, arg2 : arg2, arg3 : arg3 } )
				.always( function()
					{
						x.val( "err" );
					}
				)
				.done( function( json )
					{
						x.val( JSON.stringify( json ) );
					}
				);
		}
	);

	var monitor_update = function()
	{
		if( !( $( ".x_ws_monitor" ).prop( 'checked' ) ) ) { return; }

		$( ".x_ws_monitor_ws_status" ).val( JSON.stringify( ws_status ) )
		$( ".x_ws_monitor_ws_spec_l" ).val( JSON.stringify( ws_spec_l ) )
		$( ".x_ws_monitor_ws_spec_r" ).val( JSON.stringify( ws_spec_r ) )
		$( ".x_ws_monitor_ws_spec_t" ).val( JSON.stringify( ws_spec_t ) )
		$( ".x_ws_monitor_ws_rms_l"  ).val( JSON.stringify( ws_rms_l ) )
		$( ".x_ws_monitor_ws_rms_r"  ).val( JSON.stringify( ws_rms_r ) )
		$( ".x_ws_monitor_ws_spec_h" ).val( JSON.stringify( ws_spec_h ) )
	};


}
);