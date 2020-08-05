$(
function()
{
	// init library

	var base_folder_item	= $( "div.x_liblist_folder_item" ).clone();
	var base_file_item		= $( "div.x_liblist_file_item" ).clone();

	$( "div.x_liblist_folder_item" ).remove();
	$( "div.x_liblist_file_item" ).remove();

	var library_bc		= $( ".x_library_bc" );
	var library_bc_lif	= $( "li:first", library_bc );
	var library_hr 		= $( ".x_liblist > hr" );

	var library_page_update_lif =
		function()
		{
			$(this).nextAll().remove();
			library_page_update();
		}

	library_bc_lif.click( library_page_update_lif );

	$( ".x_dir_up" ).click(
		function()
		{
			var t = $( "li:last", library_bc );

			if( ! t.is( library_bc_lif ) )
			{
				t.remove();
				library_page_update();
			}
		}
	);

	$( ".x_dir_home" ).click(
		function()
		{
			library_bc_lif.nextAll().remove();
			library_page_update();
		}
	);

	$( ".x_dir_refresh" ).click(
		function()
		{
			library_page_update();
		}
	 );

	$( "li", library_bc ).each( function() { if( ! library_bc_lif.is( $(this) ) ) { $(this).remove(); } } );

	var library_page_cur_dir = function()
	{
		var a = $( "li > a", library_bc );
		var p = "";

		for( var i = 1 ; i < a.length ; ++i )
		{
			if ( p != "" ) { p += "/"; }
			p += a.eq(i).text();
		}

		return p;
	};

	var library_page_add = function( _n, _p )
	{
		$.getJSON( "/cmd", { cmd : "addid", arg1 : _n } )
			.done(
				function( json )
				{
				    if( json.Ok )
				    {
						var kv = $.hidamari.parse_flds( json.Ok.flds )

						if( _p && kv[ 'Id' ] )
						{
							$.getJSON( "/cmd", { cmd : "playid", arg1 : kv[ 'Id' ] } );
						}
					}
				}
			);
	}

	var library_page_clear = function()
	{
		library_hr.prevAll().remove();

		$.each( [ ".x_liblist_dir_addall", ".x_liblist_dir_addall_play" ]
		,	function( i, v )
			{
				$( v ).attr( "disabled", "disabled" );
			}
		);
	}

	var library_page_dir_addall_impl = function( _f )
	{
		var t = [];
		var a = $( ".x_liblist_file_item_add"  );
		var p = $( ".x_liblist_file_item_play" );

		if( _f && p.length != 0 )
		{
			t.push( p.eq( 0 ) );
		}
		else if( p.length != 0 )
		{
			t.push( a.eq( 0 ) );
		}

		for( var i = 1 ; i < a.length ; ++i )
		{
			t.push( a.eq( i ) );
		}

		$.each( t, function( i, v ) { v.click(); } );
	}

	$( ".x_liblist_dir_addall" ).click( 		function() { library_page_dir_addall_impl( false 	); } );
	$( ".x_liblist_dir_addall_play" ).click( 	function() { library_page_dir_addall_impl( true		); } );

	var library_page_update = function()
	{
		$.getJSON( "/cmd", { cmd : "lsinfo", arg1 : library_page_cur_dir() } )
			.always( function()
				{
					library_page_clear();
				}
			)
			.done( function( json )
				{
				    if( json.Ok )
				    {
						var list = $.hidamari.parse_list( json.Ok.flds );

						var items = [];
						var flg_file = false;

						var f_add = function( _it, _n, _p )
						{
							return function()
							{
								$.hidamari.flush_item( _it );
								library_page_add( _n, _p );
								return false;
							};
						};

						for( var i = 0 ; i < list.length ; ++i )
						{
							var k  = list[ i ][ 0 ];
							var n  = list[ i ][ 1 ];
							var kv = list[ i ][ 2 ];

							if( k == "directory" )
							{
								var folder_item = base_folder_item.clone();

								$( ".x_liblist_folder_item_title_1", folder_item ).text( n );

								folder_item.click(
									function( _n )
									{
										return function()
										{
											var li = library_bc_lif.clone();
											$( "a", li ).text( _n );
											li.click( library_page_update_lif );
											library_bc.append( li );
											li.click();
										}
									}( n )
								);

								items.push( folder_item );
							}
							else if( k == "file" )
							{
								var file_item = base_file_item.clone();

								$( ".x_liblist_file_item_title_1", file_item ).text( kv[ '_title_1' ] );
								$( ".x_liblist_file_item_title_2", file_item ).text( kv[ '_title_2' ] );
								$( ".x_liblist_file_item_time",    file_item ).text( kv[ '_time' ] );

								$( ".x_liblist_file_item_add",  file_item ).click( f_add( file_item, kv[ '_name' ], false ) );
								$( ".x_liblist_file_item_play", file_item ).click( f_add( file_item, kv[ '_name' ], true ) );


								var item_desc = $( ".x_liblist_file_item_desc", file_item );

								item_desc.data( "x_name", kv[ '_name' ] );
								item_desc.click( function() { show_description( $(this).data( "x_name" ), false ); } );

								items.push( file_item );

								flg_file |= true;
							}
						}

						for( var i = 0 ; i < items.length ; ++i )
						{
							library_hr.before( items[ i ] );
						}

						if( flg_file )
						{
							$.each( [ ".x_liblist_dir_addall", ".x_liblist_dir_addall_play" ]
							,	function( i, v )
								{
									$(v).removeAttr( "disabled" );
								}
							);
						}
					}
				}
			)
			;
	}

	library_page_update();

	// init playlist

	var base_playlist_item	= $( "div.x_playlist_item" ).clone();

	$( "div.x_playlist_item" ).remove();

	var playlist_hr 			= $( ".x_playlist > hr" );

	var playlist_check_selection = function()
	{
		var item_n = $( ".x_playlist_item_select" ).length;

		$.each( [
		 	".x_playlist_check_all"
		]
		,	function( i, v )
			{
				if( item_n )
				{
					$(v).removeAttr( "disabled" );
				}
				else
				{
					$(v).attr( "disabled", "disabled" );
				}
			}
		);

		var sel_n = $.grep
			(
				$( ".x_playlist_item_select" )
			,	function( n, i )
				{
					return !!( $(n).data( "x_selected" ) );
				}
			)
			.length;

		$.each( [
		 	".x_playlist_up"
		, 	".x_playlist_down"
		,	".x_playlist_remove"
		]
		,	function( i, v )
			{

				if( sel_n )
				{
					$(v).removeAttr( "disabled" );
				}
				else
				{
					$(v).attr( "disabled", "disabled" );
				}
			}
		);
	}

	var sel = !!( $( ".x_playlist_check_all" ).data( "x_selected" ) );
	$( ".x_playlist_check_all" ).data( "x_selected", sel );

	$( ".x_playlist_check_all" ).click(
		function()
		{
			var sel = !( $(this).data( "x_selected" ) );
			$(this).data( "x_selected", sel );

			$( ".x_playlist_item_select" ).data( "x_selected", sel );
			$( ".x_playlist_item_select_on"  ).toggle( sel );
			$( ".x_playlist_item_select_off" ).toggle( !sel );

			$.hidamari.select_item( sel, $( ".x_playlist_item" ) );

			playlist_check_selection();
		}
	);

	$( ".x_playlist_remove" ).click(
		function()
		{
			var t = $( ".x_playlist_item_select" );

			$.each( t.get().reverse(),
				function( i, v )
				{
					if( !!( $(v).data( "x_selected" ) ) )
					{
						$.getJSON( "/cmd", { cmd : "deleteid", arg1 : $(v).data( "x_id" ) },  )
					}
				}
			);

			playlist_update();
		}
	);

	var playlist_up_down = function( mode_up )
	{
		var id_pos = [];

		$.each( $( ".x_playlist_item_select" )
		,	function( i, v )
			{
				v = $( v );

				if( !!( v.data( "x_selected" ) ) )
				{
					var id  = v.data( "x_id" );
					var pos = v.data( "x_pos" );

					id_pos.push( [ id, pos ] );
				}
			}
		);

		var id_pos_m = [];

		if( mode_up )
		{
			for( var i = 0 ; i < id_pos.length ; ++i )
			{
				if( id_pos[i][1] != 0 && ( i == 0 || id_pos[i-1][1] < id_pos[i][1] - 1 ) )
				{
					id_pos[i][1]--;
					id_pos_m.push( [ id_pos[i][0], id_pos[i][1] ] );
				}
			}
		}
		else
		{
			var item_n = $( ".x_playlist_item" ).length;

			for( var i = id_pos.length - 1 ; i >= 0 ; --i )
			{
				if( id_pos[i][1] != item_n - 1 && ( i == id_pos.length - 1 || id_pos[i+1][1] > id_pos[i][1] + 1 ) )
				{
					id_pos[i][1]++;
					id_pos_m.push( [ id_pos[i][0], id_pos[i][1] ] );
				}
			}
		}

		$.each( id_pos_m
		,	function( i, v )
			{
				$.ajax(
					{
						dataType	: "json"
					,	url			: "/cmd"
					, 	data		: { cmd : "moveid", arg1 : v[0], arg2 : v[1] }
					,	async		: false
					}
				);
			}
		);

		playlist_update();
	};

	$( ".x_playlist_up" ).click(
		function()
		{
			playlist_up_down( true );
		}
	);

	$( ".x_playlist_down" ).click(
		function()
		{
			playlist_up_down( false );
		}
	);

	$( ".x_playlist_dropdown" ).on( 'show.bs.dropdown',
		function ()
		{
			$( ".dropdown-item", $(this) ).removeClass( "dropdown-item-checked" );

			if( ws.ws_status )
			{
				var d = $.hidamari.parse_flds( ws.ws_status );

				$.each( [
					"repeat"
				,	"random"
				,	"single"
				,	"consume"
				],	function( i, v )
					{
						if( d[ v ] != "0" )
						{
							$( ".x_" + v ).addClass( "dropdown-item-checked" );
						}
					}
				);
			}
		}
	);

	$.each( [
		"repeat"
	,	"random"
	,	"single"
	,	"consume"
	],	function( i, v )
		{
			$( ".x_" + v ).click(
				function( _v )
				{
					return function()
					{
						var sw = $(this).hasClass( "dropdown-item-checked" );

						$.getJSON( "/cmd", { cmd : _v, arg1 : sw ? "0" : "1" } )
					};
				}( v )
			);
		}
	);

	var current_songid 		= "";
	var current_songid_prev	= "";

	var playlist_select_song = function( songid, _f )
	{
		current_songid_prev = current_songid;
		current_songid = songid;

		var pl = $( ".x_playlist" );

		$( ".x_playlist_item" ).each(
			function()
			{
				var t = $(this);
				var s = ( songid != "" && t.data( "x_id" ) == songid );

				t.toggleClass( "text-warning x_now_play", 	s );
				t.toggleClass( "text-white",       			!s );

				if( s && ( _f || current_songid_prev != songid ) )
				{
					var st = pl.scrollTop();
					var sh = pl.innerHeight();
					var tt = t.position().top;
					var th = t.height();

					if( tt < 0  )
					{
						var v = ( st + tt - th );
					    pl.animate( { scrollTop: v } );
					}
					else if( tt > sh )
					{
						var v = st + ( tt + th - sh );
					    pl.animate( { scrollTop: v } );
					}
				}
			}
		);
	}

	var playlist_update_select_song = function()
	{
		if( ws.ws_status )
		{
			var d = $.hidamari.parse_flds( ws.ws_status );
			playlist_select_song( d[ 'songid' ], true )
		}
	}

	var playlist_update = function()
	{
		sel_id = {}

		$.each( $( ".x_playlist_item_select" )
		,	function( i, v )
			{
				v = $( v );

				if( !!( v.data( "x_selected" ) ) )
				{
					sel_id[ v.data( "x_id" ) ] = 1;
				}
			}
		);

		$.getJSON( "/cmd", { cmd : "playlistinfo" } )
			.always( function()
				{
					$.each( [
						".x_playlist_check_all"
					, 	".x_playlist_up"
					, 	".x_playlist_down"
					, 	".x_playlist_remove"
					]
					,	function( i, v )
						{
							$( v ).attr( "disabled", "disabled" );
						}
					);
				}
			)
			.done( function( json )
				{
				    if( json.Ok )
				    {
						var list = $.grep( $.hidamari.parse_list( json.Ok.flds ), function( n, i ){ return n[0] == 'file'; } );

						var items = playlist_hr.siblings( ".x_playlist_item" );

						if( list.length < items.length )
						{
							for( var i = items.length - 1 ; i >= list.length ; --i )
							{
								items.eq( i ).remove();
							}

							items = playlist_hr.siblings( ".x_playlist_item" );

						}
						else if( list.length > items.length )
						{
							for( var i = 0 ; i < ( list.length - items.length ) ; ++i )
							{
								var item = base_playlist_item.clone();

								var item_sel = $( ".x_playlist_item_select", item );

								item_sel.click(
									function( _it )
									{
										return function()
										{
											var sel = !( $(this).data( "x_selected" ) );
											$(this).data( "x_selected", sel );

											$( ".x_playlist_item_select_on",  $(this) ).toggle( sel );
											$( ".x_playlist_item_select_off", $(this) ).toggle( !sel );

											$.hidamari.select_item( sel, _it );

											playlist_check_selection();

											return false;
										}
									}( item )
								);


								item_sel.data( "x_selected", true );
								item_sel.click();

								var item_play = $( ".x_playlist_item_play", item );

								item_play.click(
									function( _it )
									{
										return function()
										{
											$.hidamari.flush_item( _it )
											$.getJSON( "/cmd", { cmd : "playid", arg1 : $(this).data( "x_id" ) },  )
											return false;
										}
									}( item )
								);

								var item_desc = $( ".x_playlist_item_desc", item );

								item_desc.click( function() { show_description( $(this).data( "x_id" ), true ); } );

								playlist_hr.before( item );
							}

							items = playlist_hr.siblings( ".x_playlist_item" );
						}

						var flg_file = false;

						for( var i = 0 ; i < list.length ; ++i )
						{
							var k  = list[ i ][ 0 ];
							var n  = list[ i ][ 1 ];
							var kv = list[ i ][ 2 ];

							var item = items.eq( i );

							$( ".x_playlist_item_pos", 	  item ).text( kv[ '_pos' ] + 1 + "." );
							$( ".x_playlist_item_title_1", item ).text( kv[ '_title_1' ] );
							$( ".x_playlist_item_title_2", item ).text( kv[ '_title_2' ] );
							$( ".x_playlist_item_time",    item ).text( kv[ '_time' ] );

							var sel = sel_id[ kv[ '_id' ] ];

							var item_sel = $( ".x_playlist_item_select", item );

							item_sel.data( "x_selected", !sel );
							item_sel.click();

							item_sel.data( "x_id",   kv[ '_id' ] );
							item_sel.data( "x_pos",  kv[ '_pos' ] );

							var item_play = $( ".x_playlist_item_play", item );

							item_play.data( "x_id",  kv[ '_id' ] );

							var item_desc = $( ".x_playlist_item_desc", item );

							item_desc.data( "x_id", kv[ '_id' ] );

							item.data( "x_id",  kv[ '_id' ] );
						}
					}
					else
					{
						playlist_hr.siblings( ".x_playlist_item" ).remove();
					}

					playlist_check_selection();
					playlist_update_select_song();
				}
			)
			.fail( function()
				{
					playlist_hr.siblings( ".x_playlist_item" ).remove();
				}
			)
			;
	};

	// init player

	var update_music_position = function( _time, _duratin )
	{
		_time		= parseInt( _time );
		_duratin	= parseInt( _duratin );

		var v_ti  = "00:00";
		var v_re  = "00:00";
		var v_du  = "00:00";
		var p_max = 0;
		var p_pos = 0;

		if( _duratin	 > 0 )
		{
			v_ti  = $.hidamari.format_time( _time );
			v_re  = $.hidamari.format_time( _duratin - _time );
			v_du  = $.hidamari.format_time( _duratin );

			p_max = _duratin;
			p_pos = _time;
		}

		var v_ti_old = $( ".x_time_t" ).text();

		$( ".x_time_t" ).text( v_ti );
		$( ".x_time_r" ).text( v_re );
		$( ".x_time_d" ).text( v_du );

		if( v_ti_old != v_ti )
		{
			var p = $( ".x_position_bar" );

			p.attr(
				{
					'aria-valuenow'	: p_pos
				,	'aria-valuemin'	: 0
				,	'aria-valuemax'	: p_max
				}
			);

			var w = '' + parseInt( ( p_max == 0 ? 0 : p_pos / p_max ) * 1000 ) / 10 + '%';

			p.css( 'width', w );
		}
	}

	var position_change = function( evt )
	{
		var w		= $(this).innerWidth();
		var np		= evt.offsetX;
		var p_max	= $( ".x_position_bar" ).attr( 'aria-valuemax' );
		var v_du		= $( ".x_time_d" ).text( v_du );

		if( w != 0 && p_max != 0 && v_du != "00:00" )
		{
			var tm = parseInt( p_max * ( np / w ) );

			console.log( tm, p_max );
			$.getJSON( "/cmd", { cmd : "seekcur", arg1 : tm } )
		}
	}

	$( ".x_position" ).bind( 'mousedown',  position_change );
	$( ".x_position" ).bind( 'touchstart', position_change );

	var disabled_player = function( _f )
	{
		if( _f != $( ".x_next" ).attr( "disabled" ) )
		{
			if( _f )
			{
				$( ".x_play_play" ).show();
				$( ".x_play_pause, .x_volicon_high, .x_volicon_low" ).hide();
				$( ".x_next, .x_play, .x_prev, .x_volmut, .x_position, .x_volup, .x_voldown" ).attr( "disabled", "disabled" );
				$( ".x_vol" ).text( "---" );

				update_music_position( 0, 0 );
			}
			else
			{
				$( ".x_next, .x_play, .x_prev, .x_volmut, .x_position, .x_volup, .x_voldown" ).removeAttr( "disabled" );
			}
		}

		if( _f )
		{
			volume = -1;
			update_volume( 0 );
		}
	};

	$( ".x_next" ).click(
		function()
		{
			$.getJSON( "/cmd", { cmd : "next" } );
		}
	);

	$( ".x_prev" ).click(
		function()
		{
			$.getJSON( "/cmd", { cmd : "previous" } );
		}
	);

	$( ".x_play" ).click(
		function()
		{
			var s = $(this).data( "x_state" );

			if( s )
			{
				var c = ( s == "stop" ? "play" : ( s == "pause" ? "pause 0" : "pause 1" ) );
				$.getJSON( "/cmd", { cmd : c } );
			}
		}
	);

	var is_mut = function()
	{
		return !( $( ".x_volmut" ).prop( 'checked' ) );
	}

	var mute_timer = null;

	var set_mute = function( mute )
	{
		if( mute_timer != null ) { return; }

		$( ".x_volmut" ).prop( 'checked', !( mute === true || mute === "1" ) );

		mute_timer = setTimeout(
			function()
			{
				mute_timer = null
			}
		, 	2000
		);
	}

	var update_volume = function( d )
	{
		var volume = parseInt( $( ".x_volval" ).text() );
		volume += d;

		if( volume >= 100 ) { volume = 100; }
		if( volume <= 0   ) { volume = 0; }

		set_volume( volume, true );

		return volume;
	}

	var set_volume = function( volume, force )
	{
		if( !force && ( volume_timer_a != null || volume_timer_b != null ) ) { return; }

		if( volume >= 100 ) { volume = 100; }
		if( volume <= 0   ) { volume = 0; }

		$( ".x_volval" ).text( volume );

		if( is_mut() || volume <= 0 )
		{
			$( ".x_volicon_high, .x_volicon_low" ).hide();
			$( ".x_volicon_mut" ).show();
		}
		else if( volume <= 50 )
		{
			$( ".x_volicon_high, .x_volicon_mut" ).hide();
			$( ".x_volicon_low" ).show();
		}
		else
		{
			$( ".x_volicon_low, .x_volicon_mut" ).hide();
			$( ".x_volicon_high" ).show();
		}
	}

	var volume_timer_a = null;
	var volume_timer_b = null;

	var volume_impl = function( d )
	{
		var volume = update_volume( d );

		$.getJSON( "/cmd", { cmd : "setvol", arg1 : volume } );

		var ct = 0;

		var f = function()
		{
			var volume = update_volume( d );

			ct++;
			if( ct >= 5 )
			{
				if( ct != 0 )
				{
					$.getJSON( "/cmd", { cmd : "setvol", arg1 : volume } );
				}

				ct = 0;
			}

			volume_timer_a = setTimeout( f, 100 );
		};

		volume_timer_a = setTimeout( f, 1000 );

		var fc = function()
		{
			clearTimeout( volume_timer_a );

			if( ct != 0 )
			{
				var volume = update_volume( 0 );
				$.getJSON( "/cmd", { cmd : "setvol", arg1 : volume } );
			}

			volume_timer_a = null;
			volume_timer_b = setTimeout(
				function()
				{
					volume_timer_b = null
				}
			, 	1500
			);
        }

		$(document).one( 'mouseup', fc );
		$(document).one( 'touchend', fc );
	}

	var volume_up = function( evt ) { volume_impl( 2 );  evt.preventDefault(); return false; }
	var volume_dw = function( evt ) { volume_impl( -2 ); evt.preventDefault(); return false; }

	$( ".x_volup" ).bind( 'touchstart', volume_up );
	$( ".x_volup" ).bind( 'mousedown',  volume_up );

	$( ".x_voldw" ).bind( 'touchstart', volume_dw );
	$( ".x_voldw" ).bind( 'mousedown',  volume_dw );

	$( ".x_volmut" ).change(
		function()
		{
			$.getJSON( "/cmd", { cmd : "setmute", arg1 : is_mut() ? 1 : 0 } );
		}
	);

	var update_player = function()
	{
		if( ws.ws_status )
		{
			var df = $.hidamari.parse_list( ws.ws_status );

			var d = df[ 0 ][ 2 ];

			disabled_player( false );

			if( d[ 'state' ] == 'play' || d[ 'state' ] == 'pause' )
			{
				if( d[ 'state' ] == 'play' )
				{
					$( ".x_play_play" ).hide();
					$( ".x_play_pause" ).show();
				}
				else
				{
					$( ".x_play_play" ).show();
					$( ".x_play_pause" ).hide();
				}

				$( ".x_next, .x_prev" ).removeAttr( "disabled" );
				update_music_position( d[ 'time' ], d[ 'duration' ] )
			}
			else
			{
				$( ".x_play_play" ).show();
				$( ".x_play_pause" ).hide();
				$( ".x_next, .x_prev" ).attr( "disabled", "disabled" );
				update_music_position( 0, 0 );
			}

			$( ".x_play" ).data( "x_state", d[ 'state' ] );

			set_volume( parseInt( d[ 'volume' ] ), false );
			set_mute( d[ 'mute' ] );

			if( current_songid != d[ 'songid' ] )
			{
				playlist_update();
				playlist_select_song( d[ 'songid' ] )
			}

			$( ".x_player_title_1" ).text( "" );
			$( ".x_player_title_2" ).text( "" );
			$( ".x_player_title_3" ).text( "" );

			if( df.length >= 2 )
			{
				var kv = df[ 1 ][ 2 ];

				$( ".x_player_title_1" ).text( kv[ '_title_1' ] );
				$( ".x_player_title_2" ).text( kv[ '_title_2' ] );

				var a = "";

				if( d[ 'audio' ] )
				{
					var aa = d[ 'audio' ].split( ':' );

					if( aa[ 0 ].match( /[0-9]+/ )  && aa[ 0 ] > 0 )
					{
						a += " " + aa[ 0 ] + "Hz";
					}

					if( aa[ 1 ].match( /[0-9]+/ ) && aa[ 1 ] > 0 )
					{
						a += " " + aa[ 1 ] + "bit";
					}

					if( aa[ 2 ].match( /[0-9]+/ ) && aa[ 2 ] > 0 )
					{
						a += " " + aa[ 2 ] + "ch";
					}
				}

				if( d[ 'bitrate' ] && d[ 'bitrate' ].match( /[0-9]+/ ) && d[ 'bitrate' ] > 0 )
				{
					a += " bitrate: " + d[ 'bitrate' ] + "Kb";
				}

				$( ".x_player_title_3" ).text( a );
			}

			if( $( ".x_playlist_item" ).length != parseInt( d[ 'playlistlength' ] ) )
			{
				playlist_update();
			}
		}
		else
		{
			disabled_player( true );
		}
	}

	var show_description = function( _n, pl )
	{
		$.getJSON( "/cmd", { cmd : pl ? 'playlistid' : 'lsinfo', arg1 : _n } )
			.done( function( json )
				{
					if( json.Ok )
					{
						var list = $.hidamari.parse_list( json.Ok.flds );

						if( list.length > 0 )
						{
							var k  = list[ 0 ][ 0 ];
							var n  = list[ 0 ][ 1 ];
							var kv = list[ 0 ][ 2 ];

							var m = $('#x_item_desc');

							$( ".modal-title", m ).text( kv[ '_title_1' ] );

							var tr_base = $( "tbody > tr:last", m );

							tr_base.hide();

							$( "tbody > tr", m ).each( function() { if( ! tr_base.is( $(this) ) ) { $(this).remove(); } } );

							var key = [
								[ "Title",  	"_title_1" ]
							,	[ "Time",  		"_time" ]
							,	[ "Artist",	 	"Artist" ]
							,	[ "Album", 		"Album" ]
							,	[ "Track", 		"Track" ]
							,	[ "Genre", 		"Genre" ]
							];

							for( var i = 0 ; i < key.length ; ++i )
							{
								if( kv[ key[ i ][1] ] )
								{
									var tr = tr_base.clone();

									$( ".x_col_k", tr ).text( key[ i ][0] );
									$( ".x_col_v", tr ).text( kv[ key[ i ][1] ] );

									tr_base.before( tr );

									tr.show();
								}
							}

							if( kv[ '_path' ] && kv[ '_path' ].includes( "://" ) )
							{
								var tr = tr_base.clone();

								$( ".x_col_k", tr ).text( "URL" );
								$( ".x_col_v", tr ).text( kv[ '_path' ] );

								tr_base.before( tr );

								tr.show();
							}

							$.getJSON( "/cmd", { cmd : "readpicture", arg1 : _n, arg2 : 0 } )
								.done( function( json )
									{
										if( json.Ok )
										{
										}
									}
								);

							$('#x_item_desc').modal();
						}
					}
				}
			);

	}

	$( '#carousel' ).on( 'slide.bs.carousel',
		function ( x )
		{
			if( x.to == 0 )
			{
				playlist_update();
			}
		}
	)

	// init top

	// Config Load

	var update_theme = function( json )
	{
		$( ".x_st_themes > option" ).remove();

		if( json.Ok && json.Ok.themes )
		{
			var t = $( ".x_st_themes" );

			for( var i = 0 ; i < json.Ok.themes.length ; ++i )
			{
				var h = "";
				h += '<option value="' + json.Ok.themes[ i ]  + '" ';
				h += ( json.Ok.themes[ i ] == json.Ok.themes ? ' selected="selected" ' : '' );
				h += '>' + json.Ok.themes[ i ] + '</option>';

				t.append( $( h ) );
			}
		}
	}

	var update_url = function( json )
	{
		$( ".x_url_list > a" ).remove();

		if( json.Ok && json.Ok.url_list )
		{
			var t = $( ".x_url_list" );

			var f = function()
			{
				$( ".x_url" ).val( $(this).text() );
			};

			for( var i = 0 ; i < json.Ok.url_list.length ; ++i )
			{
				var h = "";
				h += '<a class="dropdown-item" href="#">' + json.Ok.url_list[ i ] + ' </a>';

				var a = $( h );

				a.click( f );

				t.append( a );
			}
		}
	}

	var update_aux_in = function( json )
	{
		$( ".x_aux_in_add" ).remove();

		if( json.Ok && json.Ok.aux_in && $.isArray( json.Ok.aux_in ) )
		{
			var m = $( ".x_aux_in_add_m" );

			for( var i = 0 ; i < json.Ok.aux_in.length ; ++i )
			{
				if( json.Ok.aux_in[ i ] && json.Ok.aux_in[ i ] != "" )
				{
					var t = m.clone();
					t.removeClass( "x_aux_in_add_m" );
					t.addClass( "x_aux_in_add" );

					t.text( "AUX IN " + ( i + 1 ) );
					t.data( "x_aux_val", json.Ok.aux_in[ i ] );
					t.data( "x_aux_id", ( i + 1 ) );
					m.before( t );
					t.show();
				}
			}

			$( ".x_aux_in_add" ).click(
				function()
				{
					var id = $( this ).data( "x_aux_id" );

					if( id )
					{
						$.getJSON( "/cmd", { cmd : "addauxin", arg1 : id } )
							.done( function( json )
								{
									if( json.Ok )
									{
										var kv = $.hidamari.parse_flds( json.Ok.flds )

										if( kv[ 'Id' ] )
										{
											$.getJSON( "/cmd", { cmd : "playid", arg1 : kv[ 'Id' ] } );
										}
									}
									else if( json.Err && json.Err.msg_text )
									{
										url_add_error( url, json.Err.msg_text );
									}
								}
							);
					}

				}
			);
		}
	}

	var update_config = function()
	{
		$.getJSON( "/config" )
			.always( function()
				{
					$( ".x_st_themes > option" ).remove();
					$( ".x_url_list > a" ).remove();
				}
			)
			.done( function( json )
				{
					update_theme( json );
					update_url( json );
				}
			);
	}

	$( ".x_navbar_toggler" ).click(
		function()
		{
			update_config();
		}
	);

	var url_add_error = function( url, msg )
	{
		var t = $( ".x_url_add_err_m" ).clone();

		t.removeClass( "x_url_add_err_m" );
		t.addClass( "x_url_add_err" );

		$( ".x_url_add_err_url", t ).text( url );
		$( ".x_url_add_err_msg", t ).text( msg );

		$( ".x_url_add_err_m" ).after( t );

		t.show();
	}

	$( ".x_url_add_err_m" ).hide();

	$( ".x_url_add" ).click(
		function()
		{
			$( ".x_url_add_err" ).remove();

			var url = $( ".x_url" ).val();

			if( url )
			{
				$.getJSON( "/cmd", { cmd : "addurl", arg1 : url, arg2 : true } )
					.done( function( json )
						{
							if( json.Ok )
							{
								var kv = $.hidamari.parse_flds( json.Ok.flds )

								if( kv[ 'Id' ] )
								{
									$.getJSON( "/cmd", { cmd : "playid", arg1 : kv[ 'Id' ] } );
								}

								$( ".x_url" ).val( "" );

								update_config();
							}
							else if( json.Err && json.Err.msg_text )
							{
								url_add_error( url, json.Err.msg_text );
							}
						}
					);
			}
		}
	);

	$( ".x_testsound_add" ).click(
		function()
		{
			$.getJSON( "/cmd", { cmd : "testsound" } );
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
						location.href = "/";
					}
				);
		}
	);

	$( ".x_setting_go" ).click(
		function()
		{
			location.href = "/common/setting.html";
		}
	);

	// io_list

	$( ".x_aux_in_add_m" ).hide();
	$( ".x_bt_in_add_m" ).hide();
	$( ".x_output_m" ).hide();

	var disable_io_list_update = false;

	var io_list_update = function()
	{
		if( disable_io_list_update ) { return; }

		if( ws.ws_io_list )
		{
			$( ".x_aux_in_add" ).remove();
			$( ".x_bt_in_add" ).remove();
			$( ".x_output" ).remove();

			var m_aux_in = $( ".x_aux_in_add_m" );
			var m_bt_in  = $( ".x_bt_in_add_m" );
			var m_output = $( ".x_output_m" );

			for( var i = 0 ; i < ws.ws_io_list.length ; ++i )
			{
				var item = ws.ws_io_list[ i ];

				if( item.type == "AuxIn" || item.type == "BtIn" )
				{
					var m = ( item.type == "AuxIn" ) ? m_aux_in : m_bt_in;
					var t = m.clone();

					if( item.type == "AuxIn" )
					{
						t.removeClass( "x_aux_in_add_m" );
						t.addClass( "x_aux_in_add" );
					}
					else
					{
						t.removeClass( "x_bt_in_add_m" );
						t.addClass( "x_bt_in_add" );
					}

					t.text( item.name );
					t.data( "x_aux_url",  item.url );
					t.data( "x_aux_name", item.name );
					m.before( t );
					t.show();
				}
				else if( item.type == "MpdOut" || item.type == "BtOut" )
				{
					var t = m_output.clone();

					t.removeClass( "x_output_m" );
					t.addClass( "x_output" );

					$( ".x_output_type", t ).text( ( item.type == "MpdOut" ) ? "MPD" : "BT" );
					$( ".x_output_name", t ).text( item.name );

					$( "input.x_output_input", t ).attr( "id",  "x_output_input_" + i );
					$( "label.x_output_input", t ).attr( "for", "x_output_input_" + i );

					$( "input.x_output_input", t ).data( "x_out_url",  item.url );
					$( "input.x_output_input", t ).data( "x_out_name", item.name );
					$( "input.x_output_input", t ).prop( 'checked', item.enable );

					m_output.before( t );
					t.show();
				}
			}

			$( ".x_aux_in_add, .x_bt_in_add" ).click(
				function()
				{
					var url  = $( this ).data( "x_aux_url" );
					var name = $( this ).data( "x_aux_name" );

					if( url != "" )
					{
						$.getJSON( "/cmd", { cmd : "addauxin", arg1 : url, arg2 : name } )
							.done( function( json )
								{
									if( json.Ok )
									{
										var kv = $.hidamari.parse_flds( json.Ok.flds )

										if( kv[ 'Id' ] )
										{
											$.getJSON( "/cmd", { cmd : "playid", arg1 : kv[ 'Id' ] } );
										}
									}
									else if( json.Err && json.Err.msg_text )
									{
										url_add_error( url, json.Err.msg_text );
									}
								}
							);
					}
				}
			);

			$( "input.x_output_input" ).change(
				function ()
				{
					var url  	= $( this ).data( "x_out_url" );
					var name 	= $( this ).data( "x_out_name" );
					var sw 		= $( this ).prop( 'checked' );

					if( url != "" )
					{
						$.getJSON( "/set_output", { url : url, sw : sw } );

						setTimeout(
							function()
							{
								disable_io_list_update = false;
							}
						, 	3000
						);
					}
				}
			);
		}
	}

	// websocket

	$.hidamari.ajax_setup();

	var ws = $.hidamari.websocket();

	ws.status_update( function() { update_player(); } );
	ws.io_list_update( function() { io_list_update(); } );

	ws.open();

	$( ".x_canvas-indicator" ).click(
		function()
		{
			$( ".x_canvas-indicator" ).removeClass( "active" );
			$( this ).addClass( "active" );
			var s = $( this ).data( "slide-to" );
			$( ".x_canvas-item" ).removeClass( "active" );
			$( ".x_canvas-item" ).eq( s ).addClass( "active" );
		}
	)

	var canv0 = $( ".x_canvas_0" ).get( 0 );
	var canv1 = $( ".x_canvas_1" ).get( 0 );
	var canv2 = $( ".x_canvas_2" ).get( 0 );
	var canvS = [ canv0, canv1, canv2 ];

	$.each( canvS,
		function( i, v )
		{
			$( v ).click(
				function()
				{
					if( this.requestFullscreen )
					{
						this.requestFullscreen();
					}
				}
			);
		}
	);

	var draw0 = $.hidamari.drawfunc_simple( ws );
	var draw1 = $.hidamari.drawfunc_spec_analyzer( ws );
	var draw2 = $.hidamari.drawfunc_spec_voice( ws );

	var draw = function()
	{
		if( window.document.fullscreenElement )
		{
			var v = window.document.fullscreenElement;

			var p = $( v ).parent();
			var w1 = parseInt( p.css( 'padding-left'), 10 );
			var w2 = parseInt( p.css( 'padding-right'), 10 );
			var w = p.innerWidth() - w1 - w2 - 16;
			$( v ).attr( 'width',  w );
			$( v ).attr( 'height', w * 9 / 16 );
		}
		else
		{
			$.each( canvS,
				function( i, v )
				{
					if( v )
					{
						var p = $( v ).parent();
						var w1 = parseInt( p.css( 'padding-left'), 10 );
						var w2 = parseInt( p.css( 'padding-right'), 10 );
						var w = p.innerWidth() - w1 - w2 - 16;

						var h1 = parseInt( p.css( 'padding-top'), 10 );
						var h2 = parseInt( p.css( 'padding-bottom'), 10 );
						var h = p.innerHeight() - h1 - h2 - 32;

						var ww = $( v ).attr( 'width' );
						var hh = $( v ).attr( 'height' );

						if( ww != w )
						{
							$( v ).attr( 'width', w );
						}

						if( hh != h )
						{
							$( v ).attr( 'height', h );
						}
					}
				}
			);
		}

		draw0( canv0 );
		draw1( canv1 );
		draw2( canv2 );
		requestAnimationFrame( draw );
	}

	requestAnimationFrame( draw );
}
);
