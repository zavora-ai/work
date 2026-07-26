fn main() {
    let mut wb = zavora_xlsx::Workbook::new();
    {
        let s = wb.worksheet(0).unwrap();
        s.set_name("Summary").unwrap();
        for (col, head) in ["Month", "Units", "Base"].iter().enumerate() {
            s.write(4, col as u16, *head).unwrap();
        }
        s.write(5, 0, "July").unwrap();
        s.write(5, 1, 1240.0).unwrap();
        s.write(5, 2, 4_960_000.0).unwrap();
        s.write(6, 0, "August").unwrap();
        s.write(6, 1, 1310.0).unwrap();
        s.write(6, 2, 5_240_000.0).unwrap();
        s.write(7, 0, "September").unwrap();
        s.write(7, 1, 1455.0).unwrap();
        s.write(7, 2, 5_820_000.0).unwrap();
    }
    {
        let d = wb.add_worksheet_with_name("Detail").unwrap();
        d.write(0, 0, "Line items").unwrap();
    }
    {
        let a = wb.add_worksheet_with_name("Assumptions").unwrap();
        a.write(0, 0, "Growth").unwrap();
        a.write(0, 1, 0.12).unwrap();
    }
    wb.save("/tmp/zws-multi.xlsx").unwrap();
    println!("written");
}
