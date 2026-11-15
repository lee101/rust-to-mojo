use anyhow::{anyhow, Result};

pub fn parse_digit(c: u8) -> Result<u32> {
    if c >= 48 && c <= 57 {
        Ok((c - 48) as u32)
    } else {
        Err(anyhow!("not a digit: {}", c))
    }
}

pub fn sum_two(a: u8, b: u8) -> Result<u32> {
    let x = parse_digit(a)?;
    let y = parse_digit(b)?;
    Ok(x + y)
}

pub fn check(flag: bool) -> Result<()> {
    if !flag {
        return Err(anyhow!("flag was false"));
    }
    Ok(())
}
