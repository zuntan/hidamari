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
		$( ".x_ws_monitor_ws_bt_status" ).val( JSON.stringify( ws.ws_bt_status ) )
		$( ".x_ws_monitor_ws_bt_notice" ).val( JSON.stringify( ws.ws_bt_notice ) )
		$( ".x_ws_monitor_ws_io_list" ).val( JSON.stringify( ws.ws_io_list ) )
	};

	// bluetooth

	var bt_disable_update = false;

	var bt_command = function( cmd, aid, did, sw )
	{
		if( cmd == "" ){ return; }
		sw = !!sw;

		bt_disable_update = true;

		$.getJSON( "/bt_cmd", { cmd : cmd , aid : aid, did : did, sw : sw } )
			.done( function( json )
				{
					update_error( json, ".x_st_bt_cmd_err" );
					update_ok( json, ".x_st_bt_cmd_ok" );
				}
			);

		if( cmd != "dev_remove" )
		{
			setTimeout(
				function()
				{
					bt_disable_update = false;
				}
			, 	3000
			);
		}
		else
		{
			bt_disable_update = false;
		}
	}

	$( ".x_bt_dev_z" ).hide();

	var bt_status_update = function( ws )
	{
		if( bt_disable_update ) { return; }

		var st = ws.ws_bt_status;

		$( ".x_st_bt_enable" ).text( st.enable ? "Enable" : "Disable" );

		var t = $( ".x_st_bt_adapter" );

		if( ! t.hasClass( "x_focus" ) )
		{
			var sel_id = t.val();

			t.empty();

			for( var i = 0 ; i < st.adapter.length ; ++i )
			{
				var adpt = st.adapter[ i ];

				var h = "";
				h += '<option value="' + adpt.id  + '" ';
				h += ( adpt.id == sel_id ? ' selected="selected" ' : '' );
				h += '>' + adpt.alias + ' [' + adpt.address + '] </option>';

				t.append( $( h ) );
			}
		}

		var adpt_disabled = true;
		$( ".x_bt_dev" ).remove();

		var sel_id = t.val();

		if( sel_id != "" )
		{
			for( var i = 0 ; i < st.adapter.length ; ++i )
			{
				var adpt = st.adapter[ i ];

				if( adpt.id == sel_id )
				{
					adpt_disabled = false;

					$( ".x_st_bt_powerd" 		).prop( 'checked', adpt.powered );
					$( ".x_st_bt_pairable" 		).prop( 'checked', adpt.pairable );
					$( ".x_st_bt_discoverable" 	).prop( 'checked', adpt.discoverable );
					$( ".x_st_bt_discovering" 	).prop( 'checked', adpt.discovering );

					$( ".x_st_bt_powerd, .x_st_bt_pairable, .x_st_bt_discoverable, .x_st_bt_discovering" )
						.data( "x_bt_aid", adpt.id )
						;

					var tr_base = $( ".x_bt_dev_z" );

					for( var j = 0 ; j < adpt.device_status.length ; ++j )
					{
						var dev = adpt.device_status[ j ];

						var tr = tr_base.clone();

						tr.removeClass( "x_bt_dev_z" );
						tr.addClass( "x_bt_dev" );

						$( ".x_bt_dev_alias"	, tr ).text( dev.alias + ' [' + dev.address + ']' );
						$( ".x_bt_dev_source"	, tr ).toggle( dev.audio_source );
						$( ".x_bt_dev_sink"		, tr ).toggle( dev.audio_sink );

						$( "input.x_bt_dev_connected"	, tr ).attr( "id",  "x_bt_dev_connected_" + j );
						$( "label.x_bt_dev_connected"	, tr ).attr( "for", "x_bt_dev_connected_" + j );

						$( "input.x_bt_dev_paired"		, tr ).attr( "id",  "x_bt_dev_paired_" + j );
						$( "label.x_bt_dev_paired"		, tr ).attr( "for", "x_bt_dev_paired_" + j );

						$( "input.x_bt_dev_trusted"		, tr ).attr( "id",  "x_bt_dev_trusted_" + j );
						$( "label.x_bt_dev_trusted"		, tr ).attr( "for", "x_bt_dev_trusted_" + j );

						$( "input.x_bt_dev_blocked"		, tr ).attr( "id",  "x_bt_dev_blocked_" + j );
						$( "label.x_bt_dev_blocked"		, tr ).attr( "for", "x_bt_dev_blocked_" + j );

						$( ".x_bt_dev_connected, .x_bt_dev_paired, .x_bt_dev_trusted, .x_bt_dev_blocked, .x_bt_dev_remove", tr )
							.data( "x_bt_aid", adpt.id )
							.data( "x_bt_did", dev.id )
							;

						$( "input.x_bt_dev_connected"	, tr ).prop( 'checked', dev.connected );
						$( "input.x_bt_dev_paired"		, tr ).prop( 'checked', dev.paired );
						$( "input.x_bt_dev_paired"		, tr ).prop( 'disabled', dev.paired );
						$( "input.x_bt_dev_trusted"		, tr ).prop( 'checked', dev.trusted );
						$( "input.x_bt_dev_blocked"		, tr ).prop( 'checked', dev.blocked );

						$( "input.x_bt_dev_connected"	, tr ).change(
							function ()
							{
								bt_command( "dev_connect",	$(this).data( "x_bt_aid" ), $(this).data( "x_bt_did" ), $(this).prop( 'checked' ) );
							}
						);

						$( "input.x_bt_dev_paired"		, tr ).change(
							function ()
							{
								bt_command( "dev_pair", 	$(this).data( "x_bt_aid" ), $(this).data( "x_bt_did" ), true );
							}
						);

						$( "input.x_bt_dev_trusted"		, tr ).change(
							function ()
							{
								bt_command( "dev_trust",	$(this).data( "x_bt_aid" ), $(this).data( "x_bt_did" ), $(this).prop( 'checked' ) );
							}
						);

						$( "input.x_bt_dev_blocked"		, tr ).change(
							function ()
							{
								bt_command( "dev_block",	$(this).data( "x_bt_aid" ), $(this).data( "x_bt_did" ), $(this).prop( 'checked' ) );
							}
						);

						$( ".x_bt_dev_remove"		, tr ).click(
							function ()
							{
								bt_command( "dev_remove",	$(this).data( "x_bt_aid" ), $(this).data( "x_bt_did" ), true );
							}
						);

						tr_base.before( tr );

						tr.show();
					}

					break;
				}
			}
		}

		$( ".x_st_bt_powerd, .x_st_bt_pairable, .x_st_bt_discoverable, .x_st_bt_discovering" ).prop( 'disabled', adpt_disabled );

		if( adpt_disabled )
		{
			$( ".x_bt_dev" ).remove();
			$( ".x_st_bt_powerd, .x_st_bt_pairable, .x_st_bt_discoverable, .x_st_bt_discovering" ).prop( 'checked', false );
		}
	}

	$( ".x_st_bt_powerd"		).change(
		function ()
		{
			bt_command( "ad_power", 		$(this).data( "x_bt_aid" ), "", $(this).prop( 'checked' ) );
		}
	);

	$( ".x_st_bt_pairable"		).change(
		function ()
		{
			bt_command( "ad_pairable", 		$(this).data( "x_bt_aid" ), "", $(this).prop( 'checked' ) );
		}
	);

	$( ".x_st_bt_discoverable"	).change(
		function ()
		{
			bt_command( "ad_discoverable",	$(this).data( "x_bt_aid" ), "", $(this).prop( 'checked' ) );
		}
	);

	$( ".x_st_bt_discovering"	).change(

		function ()
		{
			bt_command( "ad_discovering",	$(this).data( "x_bt_aid" ), "", $(this).prop( 'checked' ) );
		}
	);

	$( ".x_st_bt_adapter" )
		.focusin(  function() { $(this).addClass( "x_focus" ); } )
		.focusout( function() { $(this).removeClass( "x_focus" ); } )

	// ajax

	$.hidamari.ajax_setup()

	var ws = $.hidamari.websocket();
	ws.update( function() { monitor_update( this ); } );
	ws.bt_status_update( function() { bt_status_update( this ); } );
	ws.open();

}
);