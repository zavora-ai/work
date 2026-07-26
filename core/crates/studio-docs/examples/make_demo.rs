fn main() {
    let mut doc = zavora_docx::Document::new();
    doc.add_paragraph("8. Termination").style("Heading1");
    doc.add_paragraph("8.1 Either party may terminate for material breach.");
    doc.add_paragraph("9. Confidentiality").style("Heading1");
    doc.save("/tmp/zws-demo.docx").unwrap();
    println!("written");
}
