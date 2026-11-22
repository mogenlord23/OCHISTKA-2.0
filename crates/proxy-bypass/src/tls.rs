//! Разбор первого пакета TLS (ClientHello): где в нём имя сайта (SNI) и где его резать.

fn be16(d: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_be_bytes([*d.get(at)?, *d.get(at + 1)?]) as usize)
}

/// Положение имени сайта (SNI) в пакете: `(начало, длина)`. `None` — это не ClientHello с именем
/// (или пакет неполный): такой трафик не трогаем.
pub fn sni_span(data: &[u8]) -> Option<(usize, usize)> {
    // Запись TLS: тип 0x16 (handshake), версия (2), длина (2).
    if data.len() < 5 || data[0] != 0x16 {
        return None;
    }
    let record_end = (5 + be16(data, 3)?).min(data.len());
    let d = &data[..record_end];
    let mut p = 5;
    if *d.get(p)? != 1 {
        return None; // не ClientHello
    }
    p += 4; // тип и длина рукопожатия
    p += 2 + 32; // версия и случайные данные
    p += 1 + *d.get(p)? as usize; // идентификатор сессии
    p += 2 + be16(d, p)?; // наборы шифров
    p += 1 + *d.get(p)? as usize; // методы сжатия
    let extensions_end = (p + 2 + be16(d, p)?).min(d.len());
    p += 2;
    while p + 4 <= extensions_end {
        let kind = be16(d, p)?;
        let len = be16(d, p + 2)?;
        p += 4;
        if kind == 0 {
            // server_name: длина списка (2), тип имени (1, 0 = host_name), длина имени (2), имя.
            let mut q = p + 2;
            if *d.get(q)? != 0 {
                return None;
            }
            q += 1;
            let name_len = be16(d, q)?;
            q += 2;
            return (q + name_len <= d.len()).then_some((q, name_len));
        }
        p += len;
    }
    None
}

/// Куда резать пакет: внутрь имени сайта, чтобы ни одна часть не содержала его целиком.
/// `None` — не резать (нет имени или оно слишком короткое).
pub fn split_point(data: &[u8]) -> Option<usize> {
    let (start, len) = sni_span(data)?;
    (len >= 2).then_some(start + len / 2)
}

#[cfg(test)]
pub(crate) fn client_hello(host: &str) -> Vec<u8> {
    let name = host.as_bytes();
    let mut sni = vec![0u8]; // host_name
    sni.extend((name.len() as u16).to_be_bytes());
    sni.extend(name);
    let mut list = (sni.len() as u16).to_be_bytes().to_vec();
    list.extend(sni);
    let mut ext = vec![0, 0]; // тип server_name
    ext.extend((list.len() as u16).to_be_bytes());
    ext.extend(list);
    // Ещё одно расширение до SNI, чтобы разбор шёл по списку, а не по фиксированному месту.
    let mut exts = vec![0, 0x0b, 0, 2, 1, 0]; // ec_point_formats
    exts.extend(ext);
    let mut body = vec![3, 3];
    body.extend([7u8; 32]); // случайные данные
    body.push(0); // пустой идентификатор сессии
    body.extend([0, 4, 0x13, 0x01, 0x13, 0x02]); // два набора шифров
    body.extend([1, 0]); // сжатие
    body.extend((exts.len() as u16).to_be_bytes());
    body.extend(exts);
    let mut hs = vec![1, 0];
    hs.extend((body.len() as u16).to_be_bytes());
    hs.extend(body);
    let mut rec = vec![0x16, 3, 1];
    rec.extend((hs.len() as u16).to_be_bytes());
    rec.extend(hs);
    rec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_name_inside_a_client_hello() {
        let hello = client_hello("www.youtube.com");
        let (start, len) = sni_span(&hello).unwrap();
        assert_eq!(&hello[start..start + len], b"www.youtube.com");
    }

    #[test]
    fn the_split_is_inside_the_name_so_no_part_holds_it_whole() {
        for host in ["youtube.com", "discord.gg", "x.com", "ab"] {
            let hello = client_hello(host);
            let k = split_point(&hello).unwrap();
            let (start, len) = sni_span(&hello).unwrap();
            assert!(
                k > start && k < start + len,
                "{host}: {k} not inside the name"
            );
            let (a, b) = hello.split_at(k);
            let whole = |part: &[u8]| part.windows(host.len()).any(|w| w == host.as_bytes());
            assert!(!whole(a) && !whole(b), "{host}");
            // Две части вместе дают исходный пакет без потерь.
            assert_eq!([a, b].concat(), hello);
        }
    }

    #[test]
    fn anything_else_is_left_alone() {
        assert_eq!(split_point(b"GET / HTTP/1.1\r\n\r\n"), None);
        assert_eq!(split_point(&[]), None);
        assert_eq!(split_point(&[0x16, 3, 1]), None);
        // Обрезанный пакет: имени нет.
        let hello = client_hello("youtube.com");
        assert_eq!(split_point(&hello[..40]), None);
        // Имя из одного символа не режется.
        assert_eq!(split_point(&client_hello("a")), None);
        // Не ClientHello (другой тип рукопожатия).
        let mut other = client_hello("youtube.com");
        other[5] = 2;
        assert_eq!(split_point(&other), None);
    }
}
