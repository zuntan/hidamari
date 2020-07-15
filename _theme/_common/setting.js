$(
function()
{
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

	var monitor_update = function( ws )
	{
		if( !( $( ".x_ws_monitor" ).prop( 'checked' ) ) ) { return; }

		$( ".x_ws_monitor_ws_status" ).val( JSON.stringify( ws.ws_status ) )
		$( ".x_ws_monitor_ws_spec_l" ).val( JSON.stringify( ws.ws_spec_l ) )
		$( ".x_ws_monitor_ws_spec_r" ).val( JSON.stringify( ws.ws_spec_r ) )
		$( ".x_ws_monitor_ws_spec_t" ).val( JSON.stringify( ws.ws_spec_t ) )
		$( ".x_ws_monitor_ws_rms_l"  ).val( JSON.stringify( ws.ws_rms_l ) )
		$( ".x_ws_monitor_ws_rms_r"  ).val( JSON.stringify( ws.ws_rms_r ) )
		$( ".x_ws_monitor_ws_spec_h" ).val( JSON.stringify( ws.ws_spec_h ) )
	};

	$.hidamari.ajax_setup()

	var ws = $.hidamari.websocket();
	ws.update( function() { monitor_update( this ); } );
	ws.open();
}
);