$(
function()
{
	//

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

			if( k == 'directory' || k == 'file' || k == 'playlist' )
			{
				var n = v.split("/").pop();

				if( k == 'file' )
				{
					kv[ '_title_1' ] = n;

					var t = [];
					if( kv[ 'Track' ] ) { t.push( kv[ 'Track' ] ) }
					if( kv[ 'Album' ] ) { t.push( kv[ 'Album' ] ) }
					if( kv[ 'Artist' ] ) { t.push( kv[ 'Artist' ] ) }

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

		list.reverse();

		return list;
	}

	// init

	$( ".test2" ).click(
		function()
		{
		}
	);

	var ajax_state = $( "div.x_ajax_state" );

	ajax_state.hide();

	$( document ).ajaxStart(
		function() {
			ajax_state.show();
		}
	);

	$( document ).ajaxStop(
		function() {
			ajax_state.hide();
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
							$.getJSON( "cmd", { cmd : "playid", arg1 : kv[ 'Id' ] } )
								.done(
									function( json )
									{
									}
								);
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

						var f_add = function( _n, _p )
						{
							return function()
							{
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

								$( ".x_liblist_file_item_add",  file_item ).click( f_add( kv[ '_name' ], false ) );
								$( ".x_liblist_file_item_play", file_item ).click( f_add( kv[ '_name' ], true ) );

								file_item.data( "x_id",   kv[ '_id' ] );
								file_item.data( "x_name", kv[ '_name' ] );
								file_item.click( function() { show_description( $(this).data( "x_name" ) ); } );

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

	$( ".x_playlist_check_all" ).click(
		function()
		{
			var sel = !( $(this).data( "x_selected" ) );
			$(this).data( "x_selected", sel );

			$( ".x_playlist_item_select" ).data( "x_selected", sel );
			$( ".x_playlist_item_select_on"  ).toggle( sel );
			$( ".x_playlist_item_select_off" ).toggle( !sel );

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
		var id_set = {};

		$.each( $( ".x_playlist_item_select" )
		,	function( i, v )
			{
				v = $( v );

				if( !!( v.data( "x_selected" ) ) )
				{
					var id  = v.data( "x_id" );
					var pos = v.data( "x_pos" );

					id_pos.push( [ id, pos ] );
					id_set[ id ] = 1;
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

		playlist_update( id_set );
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

	var playlist_update = function( selected )
	{
		var sel = !!( $( ".x_playlist_check_all" ).data( "x_selected" ) );
		$( ".x_playlist_check_all" ).data( "x_selected", sel );

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
									function()
									{
										var sel = !( $(this).data( "x_selected" ) );
										$(this).data( "x_selected", sel );

										$( ".x_playlist_item_select_on",  $(this) ).toggle( sel );
										$( ".x_playlist_item_select_off", $(this) ).toggle( !sel );

										playlist_check_selection();

										return false;
									}
								);

								item_sel.data( "x_selected", true );
								item_sel.click();

								var item_play = $( ".x_playlist_item_play", item );

								item_play.click(
									function()
									{
										console.log( $(this).data( "x_id" ) );

										$.getJSON( "cmd", { cmd : "playid", arg1 : $(this).data( "x_id" ) },  )
										return false;
									}
								);

								item.click( function() { show_description( $(this).data( "x_name" ) ); } );

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

							item.data( "x_name", kv[ '_name' ] );
							item.data( "x_id",   kv[ '_id' ] );

							var sel = ( selected && selected[ kv[ '_id' ] ] );

							var item_sel = $( ".x_playlist_item_select", item );

							item_sel.data( "x_selected", !sel );
							item_sel.click();

							item_sel.data( "x_id",   kv[ '_id' ] );
							item_sel.data( "x_pos",  kv[ '_pos' ] );

							var item_play = $( ".x_playlist_item_play", item );

							item_play.data( "x_id",  kv[ '_id' ] );
						}
					}
					else
					{
						playlist_hr.siblings( ".x_playlist_item" ).remove();
					}

					playlist_check_selection();
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

/*
	window.setInterval(
		function()
		{
			$.getJSON( "status" )
				.always( function()
					{
					}
				)
				.done( function( json )
					{
					    if( json.Ok )
					    {
						}
					}
				);
		}
	,	250
	);
*/

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

									console.log( tr );
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
}
);
