$(
function()
{
	// common function

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

	$( ".test2" ).click(
		function()
		{
		}
	);

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

	// init library

	var base_folder_item		= $( "div.x_liblist_folder_item" ).clone();
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

	$( ".x_dir_refresh" ).click( library_page_update );

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
		$.getJSON( "cmd", { cmd : "addid", arg1 : _n } )
			.done(
				function( json )
				{
				    if( json.Ok )
				    {
						var kv = parse_flds( json.Ok.flds )

						if( _p && kv[ 'Id' ] )
						{
							$.getJSON( "cmd", { cmd : "playid", arg1 : kv[ 'Id' ] } );
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

	$( ".x_liblist_dir_addall" ).click( 		function() { library_page_dir_addall_impl( false ); } );
	$( ".x_liblist_dir_addall_play" ).click( function() { library_page_dir_addall_impl( true ); } );

	var library_page_update = function()
	{
		$.getJSON( "cmd", { cmd : "lsinfo", arg1 : library_page_cur_dir() } )
			.always( function()
				{
					library_page_clear();
				}
			)
			.done( function( json )
				{
				    if( json.Ok )
				    {
						var list = parse_list( json.Ok.flds );

						var items = [];
						var flg_file = false;

						var f_add = function( _it, _n, _p )
						{
							return function()
							{
								flush_item( _it );
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
								item_desc.click( function() { show_description( $(this).data( "x_name" ) ); } );

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

			select_item( sel, $( ".x_playlist_item" ) );

			playlist_check_selection();
		}
	);

	$( ".x_playlist_remove" ).click(
		function()
		{
			$.each( $( ".x_playlist_item_select" ),
				function( i, v )
				{
					if( !!( $(v).data( "x_selected" ) ) )
					{
						$.getJSON( "cmd", { cmd : "deleteid", arg1 : $(v).data( "x_id" ) },  )
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
					,	url		: "cmd"
					, 	data		: { cmd : "moveid", arg1 : v[0], arg2 : v[1] }
					,	async	: false
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

			if( status_ws.Ok )
			{
				var d = parse_flds( status_ws.Ok.flds );

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

						$.getJSON( "cmd", { cmd : _v, arg1 : sw ? "0" : "1" } )
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
		if( status_ws.Ok )
		{
			var d = parse_flds( status_ws.Ok.flds );
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

		$.getJSON( "cmd", { cmd : "playlistinfo" } )
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
						var list = $.grep( parse_list( json.Ok.flds ), function( n, i ){ return n[0] == 'file'; } );

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

											select_item( sel, _it );

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
											flush_item( _it )
											$.getJSON( "cmd", { cmd : "playid", arg1 : $(this).data( "x_id" ) },  )
											return false;
										}
									}( item )
								);

								var item_desc = $( ".x_playlist_item_desc", item );

								item_desc.click( function() { show_description( $(this).data( "x_name" ) ); } );

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

							item_desc.data( "x_name", kv[ '_name' ] );

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
		_duratin		= parseInt( _duratin );

		var v_ti  = "00:00";
		var v_re  = "00:00";
		var v_du  = "00:00";
		var p_max = 0;
		var p_pos = 0;

		if( _duratin	 > 0 )
		{
			v_ti  = format_time( _time );
			v_re  = format_time( _duratin - _time );
			v_du  = format_time( _duratin );

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
			$.getJSON( "cmd", { cmd : "seekcur", arg1 : tm } )
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
			$.getJSON( "cmd", { cmd : "next" } );
		}
	);

	$( ".x_prev" ).click(
		function()
		{
			$.getJSON( "cmd", { cmd : "previous" } );
		}
	);

	$( ".x_play" ).click(
		function()
		{
			var s = $(this).data( "x_state" );

			if( s )
			{
				var c = ( s == "stop" ? "play" : ( s == "pause" ? "pause 0" : "pause 1" ) );
				$.getJSON( "cmd", { cmd : c } );
			}
		}
	);

	var volume			= -1;
	var is_mut			= function() { return !( $( ".x_volmut" ).prop( 'checked' ) ); }

	var update_volume = function( d )
	{
		if( volume < 0 )
		{
			$( ".x_volicon_high, .x_volicon_low" ).hide();
			$( ".x_volicon_mut" ).show();
			$( ".x_volval" ).text( volume );

			return -1;
		}

		var volume_old = volume;
		var mut = is_mut();

		volume += d;

		if( volume >= 100 ) { volume = 100; }
		if( volume <= 0   ) { volume = 0; }

		$( ".x_volval" ).text( volume );

		if( mut || volume <= 0 )
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

		return mut || ( volume == volume_old ) ? -1 : volume;
	}

	var volume_timer = null;

	var volume_impl = function( d )
	{
		update_volume( d );

		if( !is_mut() && volume != -1 )
		{
			$.getJSON( "cmd", { cmd : "setvol", arg1 : volume } );
		}

		var ct = 0;

		var f =	function()
		{
			update_volume( d );

			ct++;
			if( ct >= 5 )
			{
				if( ct != 0 && !is_mut() && volume != -1 )
				{
					$.getJSON( "cmd", { cmd : "setvol", arg1 : volume } );
				}

				ct = 0;
			}

			volume_timer = setTimeout( f, 100 );
		};

		volume_timer = setTimeout( f, 1000 );

		var fc = function()
		{
			clearTimeout( volume_timer );

			if( ct != 0 && !is_mut() && volume != -1 )
			{
				$.getJSON( "cmd", { cmd : "setvol", arg1 : volume } );
			}

			volume_timer = null;
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
			update_volume( 0 );
			$.getJSON( "cmd", { cmd : "setvol", arg1 : is_mut() ? 0 : volume } );
		}
	);

	var update_player = function()
	{
		if( status_ws.Ok )
		{
			var df = parse_list( status_ws.Ok.flds );

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

			if( volume_timer == null && !is_mut() )
			{
				volume = parseInt( d[ 'volume' ] );
				update_volume( 0 );
			}

			if( current_songid != d[ 'songid' ] )
			{
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

					a += " " + aa[ 0 ] + "Hz";
					a += " " + aa[ 1 ] + "bit";
					a += " " + aa[ 2 ] + "ch";
				}
				if( d[ 'bitrate' ] ) { a += " bitrate: " + d[ 'bitrate' ] + "Kb"; }

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

	var show_description = function( _n )
	{
		$.getJSON( "cmd", { cmd : "lsinfo", arg1 : _n } )
			.done( function( json )
				{
					if( json.Ok )
					{
						var list = parse_list( json.Ok.flds );

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
								[ "Title",  		"_title_1" ]
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

							$.getJSON( "cmd", { cmd : "readpicture", arg1 : _n, arg2 : 0 } )
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

	var status_ws = {};

	var ws_open = function()
	{
		var ws_proto = location.protocol == 'https:' ? 'wss:' : 'ws:';
	    var ws = new WebSocket( ws_proto + '//' + location.host + '/status_ws' );

		var ws_reopen = function()
		{
			status_ws = {};
			setTimeout( ws_open, 1000 );
		};

		ws.onclose = ws_reopen;
	    ws.onError = ws_reopen;
	    ws.onmessage = function( e )
        {
			status_ws = $.parseJSON( e.data );
			update_player();
		};
	};

	ws_open();
}
);
