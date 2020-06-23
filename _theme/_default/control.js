$(
function()
{
	//

	var parse_list = function( flds )
	{
		var list = Array();

		flds.reverse();

		var kv = {};

		for( var i = 0 ; i < flds.length ; ++i )
		{
			var k = flds[ i ][0];
			var v = flds[ i ][1];

			if( k == 'directory' || k == 'file' || k == 'playlist' )
			{
				var n = v.split("/").pop();
				list.push( Array( k, n, kv ) );
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

	var base_playlist_item	= $( "div.x_playlist_item" ).clone();
	var base_folder_item		= $( "div.x_liblist_folder_item" ).clone();
	var base_file_item		= $( "div.x_liblist_file_item" ).clone();

	$( "div.x_playlist_item" ).remove();
	$( "div.x_liblist_folder_item" ).remove();
	$( "div.x_liblist_file_item" ).remove();

	var library_bc     = $( ".x_library_bc" );
	var library_bc_lif = $( "li:first", library_bc );

	var update_library_page_lif =
		function()
		{
			$(this).nextAll().remove();
			update_library_page();
		}

	library_bc_lif.click( update_library_page_lif );

	$( ".x_dir_up" ).click(
		function()
		{
			var t = $( "li:last", library_bc );

			if( ! t.is( library_bc_lif ) )
			{
				t.remove();
				update_library_page();
			}
		}
	);

	$( ".x_dir_home" ).click(
		function()
		{
			library_bc_lif.nextAll().remove();
			update_library_page();
		}
	);

	$( ".x_dir_refresh" ).click( update_library_page );

	$( "li", library_bc ).each( function() { if( ! library_bc_lif.is( $(this) ) ) { $(this).remove(); } } );

	var cur_dir = function()
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

	var update_library_page = function()
	{
		var hr = $( ".x_liblist > hr" );

		hr.prevAll().remove();

		$.getJSON( "cmd", { cmd : "lsinfo", arg1 : cur_dir() } )
			.done( function( json )
				{
				    if( json.Ok )
				    {
						var items = Array();

						var list = parse_list( json.Ok.flds )

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
											li.click( update_library_page_lif );
											library_bc.append( li );
											li.click();
										}
									}( n )
								);

								items.push( folder_item )
							}
							else if( k == "file" )
							{
								var file_item = base_file_item.clone();

								$( ".x_liblist_file_item_title_1", file_item ).text( n );
								$( ".x_liblist_file_item_title_2", file_item ).text( "" );
								$( ".x_liblist_file_item_time",    file_item ).text( "" );

								items.push( file_item )
							}
						}

						for( var i = 0 ; i < items.length ; ++i )
						{
							hr.before( items[ i ] );
						}
					}
				}
			)
			;
	}

	update_library_page();

	$( '#carousel' ).on( 'slide.bs.carousel',
		function ( x )
		{
			console.log( x );
		}
	)
}
);
