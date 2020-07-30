$(
function()
{
	// Config Load

	$( ".x_main_return" ).click(
		function()
		{
			location.href = "/";
		}
	);

	$( ".x_err_m"	).hide();
	$( ".x_ok_m"	).hide();

	var clear_all_error = function()
	{
		$( ".x_err" ).remove();
	}

	var update_error = function( json, pos )
	{
		$( ".x_err", $( pos ) ).remove();

		if( json.Err )
		{
			var t = $( ".x_err_m" ).clone();

			t.removeClass( "x_err_m" );
			t.addClass( "x_err" );

			$( ".x_err_msg", t ).text( json.Err.err_msg );
			$( pos ).append( t );
			t.slideDown();
		}
	}

	var update_ok = function( json, pos )
	{
		$( ".x_ok", $( pos ) ).remove();

		if( json.Ok )
		{
			var t = $( ".x_ok_m" ).clone();

			t.removeClass( "x_ok_m" );
			t.addClass( "x_ok" );

			$( pos ).append( t );
			t.show();

			$.hidamari.flush_and_hide_item( t );
		}
	}

	var update_url_list = function( json )
	{
		if( json.Ok && json.Ok.url_list )
		{
			$( ".x_st_url_list" ).val( json.Ok.url_list.join( "\n" ) + "\n" );
		}
	}

	var update_aux_in = function( json )
	{
		if( json.Ok && json.Ok.aux_in )
		{
			$( ".x_st_aux_in" ).val( json.Ok.aux_in.join( "\n" ) + "\n" );
		}
	}

	var update_theme = function( json )
	{
		if( json.Ok && json.Ok.themes )
		{
			var t = $( ".x_st_themes" );

			for( var i = 0 ; i < json.Ok.themes.length ; ++i )
			{
				var h = "";
				h += '<option value="' + json.Ok.themes[ i ]  + '" ';
				h += ( json.Ok.themes[ i ] == json.Ok.theme ? ' selected="selected" ' : '' );
				h += '>' + json.Ok.themes[ i ] + '</option>';

				t.append( $( h ) );
			}
		}
	}

	var update_anidelay = function( json )
	{
		if( json.Ok )
		{
			$( ".x_st_anidelay" ).val( json.Ok.spec_delay )
			$( ".x_st_anidelay_val" ).text( json.Ok.spec_delay )
		}
	}

	var update_all = function( json )
	{
		update_url_list( json );
		update_aux_in( json );
		update_theme( json );
		update_anidelay( json );
	}

	$.getJSON( "/config" )
		.always( function()
			{
				$( ".x_st_themes > option" ).remove();
			}
		)
		.done( function( json )
			{
				update_all( json );
			}
		);

	$( ".x_st_url_list_update" ).click(
		function()
		{
			var url_list = $( ".x_st_url_list" ).val().split( "\n" );

			$.getJSON( "/config", { update : JSON.stringify( { url_list : url_list } ) } )
				.always( function()
					{
					}
				)
				.done( function( json )
					{
						update_error( json, ".x_st_url_list_err" );
						update_ok( json, ".x_st_url_list_ok" );
						update_url_list( json );
					}
				);
		}
	);

	$( ".x_st_aux_in_update" ).click(
		function()
		{
			var aux_in = $( ".x_st_aux_in" ).val().split( "\n" );

			$.getJSON( "/config", { update : JSON.stringify( { aux_in : aux_in } ) } )
				.always( function()
					{
					}
				)
				.done( function( json )
					{
						update_error( json, ".x_st_aux_in_err" );
						update_ok( json, ".x_st_aux_in_ok" );
						update_aux_in( json );
					}
				);
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
						update_ok( json, ".x_st_theme_ok" );
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
						update_ok( json, ".x_st_anidelay_ok" );
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

	$( ".x_st_dev_config_get" ).click(
		function()
		{
			$.getJSON( "/config" )
				.always( function()
					{
						$( ".x_st_dev_config_get_result" ).val( "" );
					}
				)
				.done( function( json )
					{
						update_error( json, ".x_st_dev_config_update_err" );

						if( json.Ok )
						{
							$( ".x_st_dev_config_get_result" ).val( JSON.stringify( json ) );
							update_all( json );
						}
					}
				);
		}
	);

	$( ".x_st_dev_config_update" ).click(
		function()
		{
			var update_values = $( ".x_st_dev_config_get_result" ).val();

			$.getJSON( "/config", { update : update_values } )
				.always( function()
					{
					}
				)
				.done( function( json )
					{
						update_ok( json, ".x_st_dev_config_update_ok" );
						update_error( json, ".x_st_dev_config_update_err" );

						if( json.Ok )
						{
							$( ".x_st_dev_config_get_result" ).val( JSON.stringify( json ) );
							update_all( json );
						}
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
		$( ".x_ws_monitor_bt_status" ).val( JSON.stringify( ws.ws_bt_status ) )
	};

	$.hidamari.ajax_setup()

	var ws = $.hidamari.websocket();
	ws.update( function() { monitor_update( this ); } );
	ws.open();
}
);