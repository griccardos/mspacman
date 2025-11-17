use std::cmp::Ordering;

/// Format a number with thousands separators
pub fn thousands(size: usize) -> String {
    let mut str = size.to_string();
    let mut pos = 1;
    let mut thousand_count = 0;

    while pos < str.len() {
        if (pos - thousand_count) % 3 == 0 {
            str.insert(str.len() - pos, ',');
            thousand_count += 1;
            pos += 1;
        }
        pos += 1
    }
    str
}
/// Natural sort comparison of two strings
/// So that "file2" < "file10"
/// Split into number and string tokens
/// Compare number tokens numerically, string tokens lexicographically
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let tokens_a = get_tokens(a);
    let tokens_b = get_tokens(b);

    for (a, b) in tokens_a.iter().zip(tokens_b.iter()) {
        let or = match (a, b) {
            (Token::Number(a), Token::Number(b)) => a.cmp(b),
            (Token::Number(_), Token::String(_)) => Ordering::Less,
            (Token::String(_), Token::Number(_)) => Ordering::Greater,
            (Token::String(a), Token::String(b)) => a.cmp(b),
        };
        if or != Ordering::Equal {
            return or;
        }
    }
    tokens_a.len().cmp(&tokens_b.len()) //in case different length but equal up to the shortest
}
enum Token {
    Number(u64),
    String(String),
}
fn get_tokens(s: &str) -> Vec<Token> {
    let mut tokens = vec![];
    let mut it = s.chars().peekable();
    loop {
        let Some(c) = it.next() else {
            break;
        };

        if c.is_numeric() {
            let mut s = String::from(c);
            while let Some(n) = it.peek() {
                if n.is_numeric() {
                    s.push(*n);
                    it.next();
                } else {
                    break;
                }
            }
            match s.parse::<u64>() {
                Ok(num) => tokens.push(Token::Number(num)),
                Err(_) => tokens.push(Token::String(s)),
            }
        } else {
            let mut s = String::from(c);
            while let Some(c) = it.peek() {
                if !c.is_numeric() {
                    s.push(*c);
                    it.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::String(s));
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_natural() {
        assert_eq!(natural_cmp("file2.txt", "file3.txt"), Ordering::Less);
        assert_eq!(natural_cmp("file2.txt", "file1.txt"), Ordering::Greater);
        assert_eq!(natural_cmp("file2.txt", "file10.txt"), Ordering::Less);
        assert_eq!(natural_cmp("file2.txt", "file10.t"), Ordering::Less);
        assert_eq!(natural_cmp("file2.t", "file10.txt"), Ordering::Less);
        assert_eq!(natural_cmp("file10.txt", "file2.txt"), Ordering::Greater);
        assert_eq!(natural_cmp("file10.txt", "file10.txt"), Ordering::Equal);

        assert_eq!(natural_cmp("1.1.1", "1.2.1"), Ordering::Less);
        assert_eq!(natural_cmp("1.1.2", "1.2.1"), Ordering::Less);
        assert_eq!(natural_cmp("2.1.1", "1.1.1"), Ordering::Greater);

        assert_eq!(natural_cmp("101551814", "317460852"), Ordering::Less);
        assert_eq!(natural_cmp("101551814", "10000564123"), Ordering::Less);
        assert_eq!(natural_cmp("101235555", "10406325"), Ordering::Greater);
        assert_eq!(natural_cmp("101235555", "8219"), Ordering::Greater);
    }
}
