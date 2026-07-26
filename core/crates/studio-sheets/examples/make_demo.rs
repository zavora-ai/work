fn main() {
    use zavora_xlsx::{Format, Workbook};
    let mut wb = Workbook::new();
    let bold = Format::new().bold();
    let ws = wb.worksheet(0).unwrap();
    ws.set_name("Summary").unwrap();
    for (c, h) in ["Month", "Units", "Base", "+12%"].iter().enumerate() {
        ws.write_with_format(4, c as u16, *h, &bold).unwrap();
    }
    for (i, (m, u, b)) in [
        ("July", 1240.0, 4_960_000.0),
        ("August", 1310.0, 5_240_000.0),
    ]
    .iter()
    .enumerate()
    {
        let r = 5 + i as u32;
        ws.write(r, 0, *m).unwrap();
        ws.write(r, 1, *u).unwrap();
        ws.write(r, 2, *b).unwrap();
        ws.write_formula_with_result(r, 3, &format!("C{}*1.12", r + 1), b * 1.12)
            .unwrap();
    }
    wb.save("/tmp/zws-demo.xlsx").unwrap();
    println!("written");
}
