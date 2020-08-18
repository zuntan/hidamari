$(
function()
{
	var hidamari =
	{
		drawfunc_simple : function( ws )
		{
			var st		= null;
			var play	= false;
			var title	= "";

			var imgurl		= "";
			var img 		= new Image();
			var img_ok  	= false;
			var img_w	  	= 0;
			var img_h  		= 0;

			var imgAlt 		= new Image();

			imgAlt.src		= 'data:image/svg+xml,<svg class="bi bi-music-note" viewBox="0 0 16 16" fill="white" xmlns="http://www.w3.org/2000/svg"><path d="M9 13c0 1.105-1.12 2-2.5 2S4 14.105 4 13s1.12-2 2.5-2 2.5.895 2.5 2z"/><path fill-rule="evenodd" d="M9 3v10H8V3h1z"/><path d="M8 2.82a1 1 0 0 1 .804-.98l3-.6A1 1 0 0 1 13 2.22V4L8 5V2.82z"/></svg>';

			$( img ).on( "load", function()
				{
					img_ok = true;
					img_w = img.naturalWidth;
					img_h = img.naturalHeight;
				}
			);

			ws.status_update(
				function()
				{
					var df = $.hidamari.parse_list( this.ws_status );
					var d = df[ 0 ][ 2 ];

					play = ( d[ 'state' ] == 'play' );

					if( df.length >= 2 )
					{
						var kv = df[ 1 ][ 2 ];
						title = kv[ '_title_1' ]

						var url = "/albumart/" + kv[ '_file' ];

						if( imgurl != url )
						{
							imgurl	= url;
							img_ok	= false;
							img.src = imgurl;
						}
					}
					else
					{
						title	= "";

						imgurl	= "";
						img_ok	= false;
						img.src = imgurl;
					}
				}
			);

			return function( canv )
			{
				var ctx  = canv.getContext( "2d" );

				if( !ctx ) { return; }

				var cw = canv.width;
				var ch = canv.height;

				ctx.clearRect( 0, 0, cw, ch );

				if( title )
				{
					if( !st )
					{
						st = {};
						st.pnow = null;
					}

					ctx.save();
					ctx.translate( cw / 2 , ch / 2 );

					var tw1 = 10000;
					var tw2 = 5000;
					var tw3 = 250;

					var pnow = performance.now();

					if( play && st.pnow == null )
					{
						st.pnow = pnow;
					}

					if( st.pnow != null )
					{
						var t = ( pnow - st.pnow ) % tw1 / tw1;
						ctx.rotate( 2 * Math.PI * t );

						if( !play && t < 0.01 )
						{
							st.pnow = null;
						}
					}

					var w = Math.min( cw, ch ) * 0.8;

					ctx.strokeStyle = "#fff";
					ctx.fillStyle   = "#000";

					ctx.lineWidth = w * 0.01;

					ctx.beginPath();
					ctx.arc( 0, 0, w / 2, 0, 2 * Math.PI );
					ctx.closePath()
					ctx.fill();
					ctx.stroke();

					var t1 = 2 * Math.PI * ( 220 / 360 );
					var t2 = 2 * Math.PI * ( 250 / 360 );

					ctx.lineWidth = w * 0.04;
					ctx.lineCap = "round";

					ctx.save();

					ctx.translate( dx, dy );

					/*
					if( st.pnow != null )
					{
						var t = ( pnow - st.pnow ) % tw3 / tw3;
						t = Math.cos( 2 * Math.PI * t ) * 2 / 360;
						ctx.rotate( 2 * Math.PI * t );
					}
					*/

					ctx.beginPath();
					ctx.arc( 0, 0, ( w / 2 ) - ( w / 2 * 0.20 ), t1, t2 );
					ctx.stroke();

					ctx.beginPath();
					ctx.arc( 0, 0, ( w / 2 ) - ( w / 2 * 0.40 ), t1, t2 );
					ctx.stroke();

					ctx.beginPath();
					ctx.arc( 0, 0, ( w / 2 ) - ( w / 2 * 0.60 ), t1, t2 );
					ctx.stroke();

					ctx.fillStyle   = "#fff";
					ctx.beginPath();
					ctx.arc( 0, 0, ( w / 2 ) - ( w / 2 * 0.75 ), 0, 2 * Math.PI );
					ctx.closePath()
					ctx.fill();

					ctx.fillStyle   = "#000";
					ctx.beginPath();
					ctx.arc( 0, 0, ( w / 2 ) - ( w / 2 * 0.92 ), 0, 2 * Math.PI );
					ctx.closePath()
					ctx.fill();

					ctx.restore();

					ctx.restore();

					if( img_ok && img_w > 0 && img_h > 0 )
					{
						ctx.save();

						var ww = w / 2;
						var a = img_w / img_h;
						var d =  ww / ( a > 0 ? img_w : img_h );

						ctx.translate( ( cw / 2 - w / 2 + 20 ) , ch - img_h * d / 2 - 40 );

						if( st.pnow != null )
						{
							var t = ( pnow - st.pnow ) % tw2 / tw2;
							t = ( Math.cos( 2 * Math.PI * t ) * 6 + 5 ) / 360;
							ctx.rotate( 2 * Math.PI * t );
						}

						var dx = - img_w * d / 2;
						var dy = - img_h * d / 2;
						var dw = img_w * d;
						var dh = img_h * d;

						ctx.fillStyle = "#fff";

						ctx.strokeStyle = "#fff";
						ctx.lineWidth = w * 0.02;
						ctx.lineCap = "round";
						ctx.lineJoin = "round";

						ctx.beginPath();
						ctx.rect( dx, dy, dw, dh );
						ctx.closePath();
						ctx.fill();
						ctx.stroke();

						ctx.drawImage( img, 0, 0, img_w, img_h, dx, dy, dw, dh );

						ctx.restore();
					}

					ctx.save();
					ctx.translate( cw / 2 , ch / 2 );

					if( st.pnow != null )
					{
						var t = ( pnow - st.pnow ) % tw1 / tw1;
						ctx.rotate( 2 * Math.PI * t );
					}

					var dx = w / 2 * 1.1;
					var dy = w / 2 * 1.1;

					ctx.save();

					ctx.translate( dx, dy );

					if( st.pnow != null )
					{
						var t = ( pnow - st.pnow ) % tw2 / tw2;
						t = Math.cos( 2 * Math.PI * t ) * 6 / 360;
						ctx.rotate( 2 * Math.PI * t );
					}

					ctx.drawImage( imgAlt, w / -2, w / -2, w / 2, w / 2 );

					ctx.restore();

					var fs = Math.round( w ) * 0.1;

					ctx.font      = "" + fs + "px sans-serif";
					ctx.textAlign = "center";
					ctx.textBaseline = "middle";
					ctx.strokeStyle = "#fff";
					ctx.lineWidth = 4;
					ctx.lineCap = "round";
					ctx.lineJoin = "round";

					ctx.fillStyle   = "#000";

					ctx.strokeText( title, 0, 0 );
					ctx.fillText( title, 0, 0 );

					ctx.restore();
				}
				else
				{
					st = null;
				}
			};
		}

	,	drawfunc_spec_analyzer : function( ws )
		{
			var st = null;

			return function( canv )
			{
				var ctx  = canv.getContext( "2d" );

				if( !ctx ) { return; }

				var cw = canv.width;
				var ch = canv.height;

				ctx.clearRect( 0, 0, cw, ch );

				if( ws.ws_spec_l && ws.ws_spec_r && ws.ws_spec_h )
				{
					if( !st )
					{
						st = {};
						st.l = ws.ws_spec_l.slice();
						st.r = ws.ws_spec_r.slice();
					}

					var spec_l = ws.ws_spec_l.slice();
					var spec_r = ws.ws_spec_r.slice();

					ctx.save();
					ctx.translate( cw / 2 , ch - 40 );

					var dw = ( cw * 0.8 / 2 ) / ws.ws_spec_h.length;
					var dh = ( ch * 0.9 ) / 1000;

					var pd = 5;
					var lb = -12;

					ctx.font      = "12px Arial";
					ctx.textAlign = "center";

					for( var i = 0 ; i < ws.ws_spec_h.length ; ++i )
					{
						var hz = ws.ws_spec_h[ i ];

						if( hz > 1000 )
						{
							hz = '' + Math.round( hz / 102.4 ) / 10 + 'k';
						}

						var y = ( ( i + 1 ) % 2 ) * 12;
						var xl = ( -i - 1 ) * dw;
						var xr = (  i + 1 ) * dw;

						ctx.fillStyle = "#fff";

						ctx.fillText( hz, xl + dw / 2, y );
						ctx.fillText( hz, xr + dw / 2, y );

						ctx.fillRect( xl, lb,  dw - 1, -spec_l[ i ] * dh );
						ctx.fillRect( xr, lb,  dw - 1, -spec_r[ i ] * dh );

						st.l[ i ] = Math.max( spec_l[ i ], Math.max( st.l[ i ] - pd, 0 ) );
						st.r[ i ] = Math.max( spec_r[ i ], Math.max( st.r[ i ] - pd, 0 ) );

						/*
						ctx.fillStyle = "#f00";
						*/

						ctx.fillRect( xl, lb - st.l[ i ] * dh,  dw - 1, -2 );
						ctx.fillRect( xr, lb - st.r[ i ] * dh,  dw - 1, -2 );
					}

					ctx.restore();
				}
			};
		}

	,	drawfunc_spec_voice : function( ws )
		{
			var st = null;

			return function( canv )
			{
				var ctx  = canv.getContext( "2d" );

				if( !ctx ) { return; }

				var cw = canv.width;
				var ch = canv.height;

				ctx.fillStyle = "#000";

				ctx.fillRect( 0, 0, cw, ch );

				if( ws.ws_spec_l && ws.ws_spec_r && ws.ws_spec_h )
				{
					if( !st )
					{
						st = {};
						st.sqlv = 0.0;
						st.rmsL = [ 0, 0 ];
						st.rmsR = [ 0, 0 ];
					}

					var spec_l = ws.ws_spec_l.slice();
					var spec_r = ws.ws_spec_r.slice();
					var rms_l  = ws.ws_rms_l;
					var rms_r  = ws.ws_rms_r;

					var pnow = performance.now();

					var clr = function( _a )
					{
						return 'hsla( ' + parseInt( 360 * ( pnow % 48000 ) / 48000 ) + ', 100%, 50%, ' + _a + ' )';
					}

					var f = function( isR, spec, rms, rmsS )
					{
						rmsS.push( rms );
						rmsS.shift();

						rms = rmsS.reduce( ( acc, cur ) => acc + cur ) / rmsS.length;
						rms /= 1000;

						var w = canv.width / 2;
						var h = canv.height / 2;

						var r0 = Math.min( w, h );
						var r1 = r0 * 0.95;
						var r2  = r0 * 0.5;
						var r2a = r0 * 0.45;
						var r3 = r0 * 0.4;
						var r3a = r0 * 0.35;

						ctx.save();
						ctx.translate( w , h );

						st.sqlv += 0.02 * ( rms < 0.01 ? -0.4 : 0.7 );
						st.sqlv = Math.max( 0, st.sqlv );
						st.sqlv = Math.min( 1, st.sqlv );

						var t1 = ( pnow % 60000 ) / 60000 ;
						var t2 = ( pnow % 30000 ) / 30000 ;

						var v = 0.5 * st.sqlv * jQuery.easing.swing( st.sqlv );

						var zm = 1.2 + Math.cos( 2 * Math.PI * t1 ) * v;

						ctx.transform( zm, Math.cos( 2 * Math.PI * t1 ) * v, Math.sin( 2 * Math.PI * t1 ) * v, zm, 0, 0 );

						if( isR )
						{
						}
						else
						{
							ctx.rotate( Math.PI )
						}

						ctx.rotate( 2 * Math.PI * ( pnow % 32000 ) / 32000 );

						ctx.lineJoin = 'round';
						ctx.lineCap  = 'round';

						ctx.strokeStyle = clr( 0.3 );
						ctx.lineWidth = 2;

						ctx.beginPath();
						ctx.arc( 0, 0, r3, 0, Math.PI * 0.98, true );
						ctx.stroke();

						if( rms >= 0.001 )
						{
							ctx.strokeStyle   = clr( rms );

							ctx.beginPath();
							ctx.arc( 0, 0, r3, 0, Math.PI, true );
							ctx.stroke();

							ctx.strokeStyle =  clr( 1 );
							ctx.fillStyle   =  clr( rms * 0.6 );
							ctx.lineWidth = 2;

							var rd0 = Math.PI / 2;
							var rd1 = rd0 * rms;

							ctx.save();
							ctx.rotate( -2 * Math.PI * ( pnow % 8000 ) / 8000 );

							ctx.beginPath();
							ctx.arc( 0, 0, r0 * 0.3                       , rd0 - rd1, rd0 + rd1, false );
							ctx.arc( 0, 0, r0 * 0.3 - ( r0 * 0.25 ) * rms , rd0 + rd1, rd0 - rd1, true );
							ctx.closePath();
							ctx.fill();
							ctx.stroke();
							ctx.restore();

							ctx.save();
							ctx.rotate( 2 * Math.PI * ( pnow % 8000 ) / 8000 );

							ctx.beginPath();
							ctx.arc( 0, 0, r0 * 0.06 + ( r0 * 0.25 ) * rms, rd0 - rd1, rd0 + rd1, false );
							ctx.arc( 0, 0, r0 * 0.06,                       rd0 + rd1, rd0 - rd1, true );
							ctx.closePath();
							ctx.fill();
							ctx.stroke();
							ctx.restore();
						}

						for( var i = 0; i < spec.length; ++i )
						{
							var p = i / spec.length;
							var v =  spec[ i ] / 1000.0;

							ctx.save();

							ctx.strokeStyle = clr( v );
							ctx.lineWidth = spec.length <= 10 ? 9 : 4;

							ctx.rotate( Math.PI * p );

							ctx.beginPath();
							ctx.moveTo( 0, r3a );
							ctx.lineTo( 0, r3a + 1 );
							ctx.stroke();

							ctx.beginPath();
							ctx.moveTo( 0, r2 );
							ctx.lineTo( 0, r2 + v  * ( r1 - r2 ) );
						    ctx.stroke();

							ctx.beginPath();
							ctx.moveTo( 0, r1 );
							ctx.lineTo( 0, r1 + 1 );
						    ctx.stroke();

							ctx.beginPath();
							ctx.moveTo( 0, r2a );
							ctx.lineTo( 0, r2a + 1 );
						    ctx.stroke();

							ctx.restore();
						}

						ctx.restore();
					}

					f( 0, spec_l, rms_l, st.rmsL );
					f( 1, spec_r, rms_r, st.rmsR );
				}
			};
		}

	,	drawfunc_spec_jagged : function( ws )
		{
			var st = {};

			return function( canv )
			{
				$( canv ).css( "background", "#000" );

				var ctx  = canv.getContext( "2d" );

				if( !ctx ) { return; }

				var pnow = performance.now();

				var cw = canv.width;
				var ch = canv.height;

				var w = cw / 2;
				var h = ch / 2;

				if( st.ids && st.cw == cw && st.ch == ch )
				{
					var ids = st.ids;

					var idd = ctx.createImageData( cw, ch );

					var p = function( xx, yy )
					{
						return ( y * cw + x ) * 4;
					}

					var pp1 = Math.sin( 2 * Math.PI * ( pnow % 17000 ) / 17000 );
					var pp2 = Math.sin( 2 * Math.PI * ( pnow % 29000 ) / 29000 );

					for( var y = 1 ; y < ch - 1 ; ++y )
					{
						for( var x = 1 ; x < cw - 1 ; ++x )
						{
							var p0 = p( x, y );

							var pt = p( x, y - 1 );
							var pb = p( x, y + 1 );
							var pl = p( x - 1, y );
							var pr = p( x + 1, y );

							idd.data[ p0     ] = ( ids.data[ pt     ] + ids.data[ pb     ] + ids.data[ pl     ] + ids.data[ pr     ] ) / 4.02 + 0.3 * pp1;
							idd.data[ p0 + 1 ] = ( ids.data[ pt + 1 ] + ids.data[ pb + 1 ] + ids.data[ pl + 1 ] + ids.data[ pr + 1 ] ) / 4.02 + 0.3 * pp1;
							idd.data[ p0 + 2 ] = ( ids.data[ pt + 2 ] + ids.data[ pb + 2 ] + ids.data[ pl + 2 ] + ids.data[ pr + 2 ] ) / 4.02 + 0.3 * pp1;
							idd.data[ p0 + 3 ] = 128 + 128 * pp2;
						}
					}

					ctx.putImageData( idd, 0, 0 );
				}

				if( ws.ws_spec_l && ws.ws_spec_r && ws.ws_spec_h )
				{
					var spec_l = ws.ws_spec_l.slice();
					var spec_r = ws.ws_spec_r.slice();
					var rms_l  = ws.ws_rms_l;
					var rms_r  = ws.ws_rms_r;

					var clr = function( _a )
					{
						return 'hsla( ' + parseInt( 360 * ( pnow % 48000 ) / 48000 ) + ', 100%, 50%, ' + _a + ' )';
					}

					var f = function( isR, spec, rms )
					{
						ctx.save();
						ctx.translate( w , h );
						ctx.rotate( 2 * Math.PI * ( pnow % 16000 ) / 16000 );

						if( isR == 0 )
						{
						}
						else if( isR == 1 )
						{
							ctx.rotate( Math.PI * 2 / 3 )
						}
						else if( isR == 2 )
						{
							ctx.rotate( Math.PI * 4 / 3 )
						}

						rms /= 1000;
						rms = Math.max( 0.25, rms );

						var r0 = Math.max( w, h );

						var m1 = 2 * Math.PI * ( pnow % 20000 ) / 20000;
						var m2 = Math.sin( 2 * Math.PI * ( pnow % 30000 ) / 30000 );
						ctx.translate( Math.cos( m1 ) * r0 * 0.15 * m2, Math.sin( m1 ) * r0 * 0.15 * m2 );

						var r1 = r0 * 1.2;
						var r2 = r0 * 0.07 * rms;
						var rd = ( r1 - r2 ) / spec.length;

						ctx.strokeStyle =  clr( 1 );
						ctx.fillStyle   =  clr( rms * 0.35 );
						ctx.lineWidth = 2;

						ctx.beginPath();
						ctx.moveTo( 0, 0 );
						ctx.arc( 0, 0, r2, Math.PI * ( 0.5 + 1.2 * rms ), Math.PI * ( 0.5 - 1.2 * rms ), true );
						ctx.closePath();
						ctx.fill();
						ctx.stroke();

						var xx = Array( spec.length );
						var yy = Array( spec.length );

						for( var i = 0 ; i < spec.length ; ++i )
						{
							yy[ i ] = ( r2 + rd * ( i + 1 ) ) * -1;
							xx[ i ] = r2 * 1.5 * spec[ i ] / 1000 * ( Math.random() * 3 - 1 );
						}

						for( var j = 0 ; j < ( rms > 0.25 ? 4 : 1 ) ; ++j )
						{
							ctx.strokeStyle =  clr( rms > 0.5 ? 1 : 0.5 );
							ctx.lineWidth 	= 1;

							ctx.beginPath();
							ctx.moveTo( 0, r2 * -0.5 );

							for( var i = 0 ; i < xx.length ; ++i )
							{
								ctx.lineTo( xx[ i ] * 0.20 * ( j + 1 ), yy[ i ] );
							}

							ctx.stroke();
						}

						if( rms > 0.25 )
						{
							ctx.strokeStyle =  clr( 1 );
							ctx.lineWidth 	= 2;

							ctx.beginPath();
							ctx.moveTo( 0, r2 * -0.5 );

							for( var i = 0 ; i < xx.length ; ++i )
							{
								ctx.lineTo( xx[ i ] , yy[ i ] );
							}

							ctx.stroke();

							ctx.strokeStyle =  clr( 1 );
							ctx.lineWidth 	= 1;

							for( var i = 0 ; i < xx.length ; ++i )
							{
								if ( Math.random() < 0.01 )
								{
									var l = Math.random() * 3;

									ctx.beginPath();
									ctx.moveTo( xx[ i ] * 2, yy[ i ] )
									ctx.lineTo( xx[ i ] * ( 2 + l ), yy[ i ] );
									ctx.stroke();
								}
							}
						}

						ctx.restore();
					}

					f( 0, spec_l, rms_l );
					f( 1, spec_r, rms_r );

					var spec_lr = Array( spec_l.length );

					for( var i = 0 ; i < spec_l.length ; ++i )
					{
						spec_lr[ i ] = ( spec_l[ i ] + spec_r[ i ] ) / 2;
					}

					f( 2, spec_lr, ( rms_l + rms_r ) / 2 );

					st.cw = cw;
					st.ch = ch;
					st.ids = ctx.getImageData( 0, 0, cw, ch );
				}
			}
		}
	}

	$.extend( { hidamari : $.extend( hidamari, $.hidamari || {} ) } );
}
);

