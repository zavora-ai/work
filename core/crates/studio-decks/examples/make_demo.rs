fn main() {
    use zavora_slide::{Emu, Layout, Presentation};
    let mut p = Presentation::new();
    for title in ["Revenue by region — Q3", "What we are asking for"] {
        let i = p.add_slide(Layout::Blank);
        p.slide_mut(i).unwrap().add_text_box(
            title,
            Emu(914_400),
            Emu(914_400),
            Emu(6_400_800),
            Emu(1_000_000),
        );
    }
    p.save("/tmp/zws-demo.pptx").unwrap();
    println!("written");
}
