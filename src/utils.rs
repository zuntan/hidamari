//  vim:set ts=4 sw=4 sts=0 fileencoding=utf-8:
//  ----------------------------------------------------------------------------
/*
    @author     zuntan
*/

///
///

pub fn url_decode_utf8( s : &str ) -> String
{
    let mut buf = Vec::< u8 >::new();

    let mut cs = s.chars();

    loop
    {
        match cs.next()
        {
            Some( c ) =>
            {
                if c == '%'
                {
                    let lb = cs.next();
                    let tb = cs.next();

                    if lb.is_some() && tb.is_some()
                    {
                        if let Ok( x ) = u8::from_str_radix( &format!( "{}{}", lb.unwrap(), tb.unwrap() ), 16 )
                        {
                            buf.push( x );
                        }
                        else
                        {
                            break;
                        }
                    }
                    else
                    {
                        break;
                    }
                }
                else
                {
                    buf.push( c as u8 );
                }
            }
        ,   None => { break; }
        }
    }

    String::from_utf8_lossy( &buf[..] ).to_string()
}
