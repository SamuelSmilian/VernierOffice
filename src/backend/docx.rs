use crate::backend::{Backend, Error};
use crate::ir::{
    BlockElement, Document, InlineElement, List, Metadata, Row, Table,
};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::{self, Write};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub struct DocxBackend;

impl Backend for DocxBackend {
    fn compile<W: io::Write + io::Seek>(&self, document: &Document, writer: W) -> Result<(), Error> {
        let mut zip = ZipWriter::new(writer);
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(content_types_xml().as_bytes())?;

        zip.start_file("_rels/.rels", options)?;
        zip.write_all(rels_xml().as_bytes())?;

        zip.start_file("docProps/core.xml", options)?;
        zip.write_all(core_xml(&document.metadata).as_bytes())?;

        zip.start_file("docProps/app.xml", options)?;
        zip.write_all(app_xml().as_bytes())?;

        zip.start_file("word/styles.xml", options)?;
        zip.write_all(styles_xml().as_bytes())?;

        zip.start_file("word/settings.xml", options)?;
        zip.write_all(settings_xml().as_bytes())?;

        let mut doc_xml = Vec::new();
        {
            let cursor = io::Cursor::new(&mut doc_xml);
            write_document_xml(document, cursor)?;
        }
        zip.start_file("word/document.xml", options)?;
        zip.write_all(&doc_xml)?;

        zip.start_file("word/_rels/document.xml.rels", options)?;
        zip.write_all(document_rels_xml().as_bytes())?;

        if !document.footnotes.is_empty() {
            let mut fn_xml = Vec::new();
            {
                let cursor = io::Cursor::new(&mut fn_xml);
                write_footnotes_xml(document, cursor)?;
            }
            zip.start_file("word/footnotes.xml", options)?;
            zip.write_all(&fn_xml)?;
        }

        zip.finish()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Static XML fragments (OOXML boilerplate)
// ---------------------------------------------------------------------------

fn content_types_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/footnotes.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
</Types>"#.to_string()
}

fn rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#.to_string()
}

fn document_rels_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes" Target="footnotes.xml"/>
</Relationships>"#.to_string()
}

fn core_xml(metadata: &Metadata) -> String {
    let title = metadata.title.as_deref().unwrap_or("");
    let author = metadata.author.as_deref().unwrap_or("");
    let date = metadata.date.as_deref().unwrap_or("");
    let language = metadata.language.as_deref().unwrap_or("");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/"
                   xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>{title}</dc:title>
  <dc:creator>{author}</dc:creator>
  <dcterms:created xsi:type="dcterms:W3CDTF">{date}</dcterms:created>
  <dc:language>{language}</dc:language>
</cp:coreProperties>"#
    )
}

fn app_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
  <Application>VernierOffice</Application>
</Properties>"#.to_string()
}

fn styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:pPr>
      <w:spacing w:after="160" w:line="276" w:lineRule="auto"/>
    </w:pPr>
    <w:rPr>
      <w:sz w:val="24"/>
      <w:szCs w:val="24"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:pPr>
      <w:spacing w:before="480" w:after="200"/>
      <w:outlineLvl w:val="0"/>
    </w:pPr>
    <w:rPr>
      <w:b/>
      <w:sz w:val="36"/>
      <w:szCs w:val="36"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:pPr>
      <w:spacing w:before="360" w:after="160"/>
      <w:outlineLvl w:val="1"/>
    </w:pPr>
    <w:rPr>
      <w:b/>
      <w:sz w:val="28"/>
      <w:szCs w:val="28"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:basedOn w:val="Normal"/>
    <w:next w:val="Normal"/>
    <w:pPr>
      <w:spacing w:before="240" w:after="120"/>
      <w:outlineLvl w:val="2"/>
    </w:pPr>
    <w:rPr>
      <w:b/>
      <w:sz w:val="24"/>
      <w:szCs w:val="24"/>
    </w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="ListParagraph">
    <w:name w:val="List Paragraph"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr>
      <w:ind w:left="720"/>
    </w:pPr>
  </w:style>
</w:styles>"#.to_string()
}

fn settings_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:defaultTabStop w:val="720"/>
  <w:characterSpacingControl w:val="compressPunctuation"/>
</w:settings>"#.to_string()
}

// ---------------------------------------------------------------------------
// document.xml
// ---------------------------------------------------------------------------

fn write_document_xml(document: &Document, writer: impl io::Write) -> Result<(), Error> {
    let mut w = Writer::new_with_indent(writer, b' ', 2);

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let xmlns_w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let xmlns_r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    w.write_event(Event::Start(
        BytesStart::new("w:document")
            .with_attributes([("xmlns:w", xmlns_w), ("xmlns:r", xmlns_r)]),
    ))?;

    w.write_event(Event::Start(BytesStart::new("w:body")))?;

    for block in &document.body {
        write_block(&mut w, block)?;
    }

    if !document.footnotes.is_empty() {
        // Empty paragraph before footnote separator
        w.write_event(Event::Start(BytesStart::new("w:p")))?;
        write_run_empty(&mut w, " ");
        w.write_event(Event::End(BytesEnd::new("w:p")))?;

        w.write_event(Event::Start(BytesStart::new("w:p")))?;
        write_run_empty(&mut w, "");
        w.write_event(Event::Start(BytesStart::new("w:r")))?;
        w.write_event(Event::Empty(BytesStart::new("w:separator")))?;
        w.write_event(Event::End(BytesEnd::new("w:r")))?;
        w.write_event(Event::End(BytesEnd::new("w:p")))?;
    }

    w.write_event(Event::Start(BytesStart::new("w:sectPr")))?;
    w.write_event(Event::Empty(
        BytesStart::new("w:pgSz").with_attributes([("w:w", "12240"), ("w:h", "15840")]),
    ))?;
    w.write_event(Event::Empty(
        BytesStart::new("w:pgMar").with_attributes([
            ("w:top", "1440"),
            ("w:right", "1440"),
            ("w:bottom", "1440"),
            ("w:left", "1440"),
            ("w:header", "720"),
            ("w:footer", "720"),
            ("w:gutter", "0"),
        ]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("w:sectPr")))?;

    w.write_event(Event::End(BytesEnd::new("w:body")))?;
    w.write_event(Event::End(BytesEnd::new("w:document")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// footnotes.xml
// ---------------------------------------------------------------------------

fn write_footnotes_xml(document: &Document, writer: impl io::Write) -> Result<(), Error> {
    let mut w = Writer::new_with_indent(writer, b' ', 2);

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;

    let xmlns_w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    w.write_event(Event::Start(
        BytesStart::new("w:footnotes").with_attributes([("xmlns:w", xmlns_w)]),
    ))?;

    write_footnote_sep(&mut w, "0")?;
    write_footnote_sep(&mut w, "-1")?;

    for footnote in &document.footnotes {
        write_footnote_entry(&mut w, &footnote.id, &footnote.content)?;
    }

    w.write_event(Event::End(BytesEnd::new("w:footnotes")))?;
    Ok(())
}

fn write_footnote_sep(w: &mut Writer<impl io::Write>, id: &str) -> Result<(), Error> {
    w.write_event(Event::Start(
        BytesStart::new("w:footnote").with_attributes([("w:id", id)]),
    ))?;
    w.write_event(Event::Start(BytesStart::new("w:p")))?;
    w.write_event(Event::Start(BytesStart::new("w:r")))?;
    w.write_event(Event::Empty(BytesStart::new("w:footnoteRef")))?;
    w.write_event(Event::End(BytesEnd::new("w:r")))?;
    w.write_event(Event::End(BytesEnd::new("w:p")))?;
    w.write_event(Event::End(BytesEnd::new("w:footnote")))?;
    Ok(())
}

fn write_footnote_entry(
    w: &mut Writer<impl io::Write>,
    id: &str,
    content: &[InlineElement],
) -> Result<(), Error> {
    w.write_event(Event::Start(
        BytesStart::new("w:footnote").with_attributes([("w:id", id)]),
    ))?;
    w.write_event(Event::Start(BytesStart::new("w:p")))?;

    w.write_event(Event::Start(BytesStart::new("w:r")))?;
    w.write_event(Event::Empty(BytesStart::new("w:footnoteRef")))?;
    w.write_event(Event::End(BytesEnd::new("w:r")))?;

    write_run_with_text(w, " ");

    for element in content {
        write_inline(w, element)?;
    }

    w.write_event(Event::End(BytesEnd::new("w:p")))?;
    w.write_event(Event::End(BytesEnd::new("w:footnote")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Block elements
// ---------------------------------------------------------------------------

fn write_block(w: &mut Writer<impl io::Write>, block: &BlockElement) -> Result<(), Error> {
    match block {
        BlockElement::Heading(h) => {
            let style = format!("Heading{}", h.level.min(3));
            w.write_event(Event::Start(BytesStart::new("w:p")))?;
            w.write_event(Event::Start(BytesStart::new("w:pPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:pStyle").with_attributes([("w:val", style.as_str())]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:pPr")))?;

            for element in &h.content {
                write_inline(w, element)?;
            }
            w.write_event(Event::End(BytesEnd::new("w:p")))?;
        }
        BlockElement::Paragraph(p) => {
            w.write_event(Event::Start(BytesStart::new("w:p")))?;
            for element in &p.content {
                write_inline(w, element)?;
            }
            w.write_event(Event::End(BytesEnd::new("w:p")))?;
        }
        BlockElement::TableOfContents(toc) => {
            w.write_event(Event::Start(BytesStart::new("w:p")))?;

            let depth = toc.depth.map(|d| d.to_string()).unwrap_or_else(|| "3".to_string());
            let instr = format!(r#" TOC \o "1-{}""#, depth);

            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(
                BytesStart::new("w:fldChar").with_attributes([("w:fldCharType", "begin")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:fldChar")))?;
            w.write_event(Event::End(BytesEnd::new("w:r")))?;

            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(
                BytesStart::new("w:instrText").with_attributes([("xml:space", "preserve")]),
            ))?;
            w.write_event(Event::Text(BytesText::new(&instr)))?;
            w.write_event(Event::End(BytesEnd::new("w:instrText")))?;
            w.write_event(Event::End(BytesEnd::new("w:r")))?;

            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(
                BytesStart::new("w:fldChar").with_attributes([("w:fldCharType", "separate")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:fldChar")))?;
            w.write_event(Event::End(BytesEnd::new("w:r")))?;

            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(
                BytesStart::new("w:fldChar").with_attributes([("w:fldCharType", "end")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:fldChar")))?;
            w.write_event(Event::End(BytesEnd::new("w:r")))?;

            w.write_event(Event::End(BytesEnd::new("w:p")))?;
        }
        BlockElement::List(list) => {
            write_list(w, list)?;
        }
        BlockElement::BlockQuote(bq) => {
            for block in &bq.content {
                w.write_event(Event::Start(BytesStart::new("w:p")))?;
                w.write_event(Event::Start(BytesStart::new("w:pPr")))?;
                w.write_event(Event::Empty(
                    BytesStart::new("w:ind")
                        .with_attributes([("w:left", "720"), ("w:right", "720")]),
                ))?;
                w.write_event(Event::End(BytesEnd::new("w:pPr")))?;

                if let BlockElement::Paragraph(p) = block {
                    for element in &p.content {
                        write_inline(w, element)?;
                    }
                }
                w.write_event(Event::End(BytesEnd::new("w:p")))?;
            }
        }
        BlockElement::CodeBlock(cb) => {
            w.write_event(Event::Start(BytesStart::new("w:p")))?;
            w.write_event(Event::Start(BytesStart::new("w:pPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:pStyle").with_attributes([("w:val", "Normal")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:pPr")))?;

            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(BytesStart::new("w:rPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:rFonts")
                    .with_attributes([("w:ascii", "Consolas"), ("w:hAnsi", "Consolas")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:rPr")))?;
            w.write_event(Event::Start(
                BytesStart::new("w:t").with_attributes([("xml:space", "preserve")]),
            ))?;
            w.write_event(Event::Text(BytesText::new(&cb.content)))?;
            w.write_event(Event::End(BytesEnd::new("w:t")))?;
            w.write_event(Event::End(BytesEnd::new("w:r")))?;
            w.write_event(Event::End(BytesEnd::new("w:p")))?;
        }
        BlockElement::Figure(fig) => {
            if let Some(caption) = &fig.caption {
                w.write_event(Event::Start(BytesStart::new("w:p")))?;
                w.write_event(Event::Start(BytesStart::new("w:pPr")))?;
                w.write_event(Event::Empty(
                    BytesStart::new("w:jc").with_attributes([("w:val", "center")]),
                ))?;
                w.write_event(Event::End(BytesEnd::new("w:pPr")))?;

                write_run_with_text(w, "[Image: ");
                write_run_with_text(w, &fig.image.path);
                write_run_with_text(w, "] ");
                for element in caption {
                    write_inline(w, element)?;
                }
                w.write_event(Event::End(BytesEnd::new("w:p")))?;
            }
        }
        BlockElement::Table(tbl) => {
            write_table(w, tbl)?;
        }
        BlockElement::Slide(_) => {
            // Slides are for PPTX only; skip in DOCX output
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

fn write_list(w: &mut Writer<impl io::Write>, list: &List) -> Result<(), Error> {
    for (i, item) in list.items.iter().enumerate() {
        for block in &item.content {
            w.write_event(Event::Start(BytesStart::new("w:p")))?;
            w.write_event(Event::Start(BytesStart::new("w:pPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:ind")
                    .with_attributes([("w:left", "720"), ("w:hanging", "360")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:pPr")))?;

            let marker = if list.ordered {
                format!("{}. ", i + 1)
            } else {
                "\u{2022} ".to_string()
            };
            write_run_with_text(w, &marker);

            if let BlockElement::Paragraph(p) = block {
                for element in &p.content {
                    write_inline(w, element)?;
                }
            }

            w.write_event(Event::End(BytesEnd::new("w:p")))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

fn write_table(w: &mut Writer<impl io::Write>, table: &Table) -> Result<(), Error> {
    if let Some(caption) = &table.caption {
        w.write_event(Event::Start(BytesStart::new("w:p")))?;
        w.write_event(Event::Start(BytesStart::new("w:pPr")))?;
        w.write_event(Event::Empty(
            BytesStart::new("w:jc").with_attributes([("w:val", "center")]),
        ))?;
        w.write_event(Event::End(BytesEnd::new("w:pPr")))?;

        write_run_with_text(w, "Table: ");
        for element in caption {
            write_inline(w, element)?;
        }
        w.write_event(Event::End(BytesEnd::new("w:p")))?;
    }

    w.write_event(Event::Start(BytesStart::new("w:tbl")))?;

    w.write_event(Event::Start(BytesStart::new("w:tblPr")))?;
    w.write_event(Event::Empty(
        BytesStart::new("w:tblW").with_attributes([("w:w", "5000"), ("w:type", "pct")]),
    ))?;

    w.write_event(Event::Start(BytesStart::new("w:tblBorders")))?;
    for border in &["top", "left", "bottom", "right", "insideH", "insideV"] {
        let tag = format!("w:{}", border);
        w.write_event(Event::Empty(
            BytesStart::new(tag.as_str()).with_attributes([
                ("w:val", "single"),
                ("w:sz", "4"),
                ("w:space", "0"),
                ("w:color", "auto"),
            ]),
        ))?;
    }
    w.write_event(Event::End(BytesEnd::new("w:tblBorders")))?;
    w.write_event(Event::End(BytesEnd::new("w:tblPr")))?;

    for row in &table.header {
        write_table_row(w, row, true)?;
    }
    for row in &table.body {
        write_table_row(w, row, false)?;
    }

    w.write_event(Event::End(BytesEnd::new("w:tbl")))?;
    Ok(())
}

fn write_table_row(
    w: &mut Writer<impl io::Write>,
    row: &Row,
    is_header: bool,
) -> Result<(), Error> {
    w.write_event(Event::Start(BytesStart::new("w:tr")))?;

    for cell in &row.cells {
        w.write_event(Event::Start(BytesStart::new("w:tc")))?;
        w.write_event(Event::Start(BytesStart::new("w:tcPr")))?;

        if cell.colspan > 1 {
            let cs = cell.colspan.to_string();
            w.write_event(Event::Empty(
                BytesStart::new("w:gridSpan").with_attributes([("w:val", cs.as_str())]),
            ))?;
        }
        if cell.rowspan > 1 {
            w.write_event(Event::Empty(
                BytesStart::new("w:vMerge").with_attributes([("w:val", "restart")]),
            ))?;
        }
        w.write_event(Event::End(BytesEnd::new("w:tcPr")))?;

        for block in &cell.content {
            if let BlockElement::Paragraph(p) = block {
                w.write_event(Event::Start(BytesStart::new("w:p")))?;

                if is_header {
                    w.write_event(Event::Start(BytesStart::new("w:pPr")))?;
                    w.write_event(Event::Empty(
                        BytesStart::new("w:jc").with_attributes([("w:val", "center")]),
                    ))?;
                    w.write_event(Event::End(BytesEnd::new("w:pPr")))?;
                }

                for element in &p.content {
                    write_inline(w, element)?;
                }
                w.write_event(Event::End(BytesEnd::new("w:p")))?;
            }
        }

        w.write_event(Event::End(BytesEnd::new("w:tc")))?;
    }

    w.write_event(Event::End(BytesEnd::new("w:tr")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Inline elements
// ---------------------------------------------------------------------------

fn write_inline(w: &mut Writer<impl io::Write>, element: &InlineElement) -> Result<(), Error> {
    match element {
        InlineElement::Text(text) => {
            write_run_with_text(w, text);
        }
        InlineElement::Emphasis(em) => {
            let bold = em.level >= 2;
            let italic = em.level % 2 == 1;
            write_run_with_formatting(w, &em.content, bold, italic, false, false)?;
        }
        InlineElement::Subscript(content) => {
            write_run_with_formatting(w, content, false, false, false, true)?;
        }
        InlineElement::Superscript(content) => {
            write_run_with_formatting(w, content, false, false, true, false)?;
        }
        InlineElement::Link(link) => {
            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(BytesStart::new("w:rPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:u").with_attributes([("w:val", "single")]),
            ))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:color").with_attributes([("w:val", "0563C1")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:rPr")))?;
            for child in &link.content {
                write_inline_plain(w, child)?;
            }
            w.write_event(Event::End(BytesEnd::new("w:r")))?;
        }
        InlineElement::InlineCode(code) => {
            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(BytesStart::new("w:rPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:rFonts")
                    .with_attributes([("w:ascii", "Consolas"), ("w:hAnsi", "Consolas")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:rPr")))?;
            w.write_event(Event::Start(
                BytesStart::new("w:t").with_attributes([("xml:space", "preserve")]),
            ))?;
            w.write_event(Event::Text(BytesText::new(code)))?;
            w.write_event(Event::End(BytesEnd::new("w:t")))?;
            w.write_event(Event::End(BytesEnd::new("w:r")))?;
        }
        InlineElement::Citation(key) => {
            let text = format!("[{}]", key);
            write_run_with_text(w, &text);
        }
        InlineElement::CrossReference(label) => {
            let text = format!("§{}", label);
            write_run_with_text(w, &text);
        }
        InlineElement::FootnoteReference(id) => {
            w.write_event(Event::Start(BytesStart::new("w:r")))?;
            w.write_event(Event::Start(BytesStart::new("w:rPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:vertAlign").with_attributes([("w:val", "superscript")]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:rPr")))?;
            w.write_event(Event::Empty(
                BytesStart::new("w:footnoteReference").with_attributes([("w:id", id.as_str())]),
            ))?;
            w.write_event(Event::End(BytesEnd::new("w:r")))?;
        }
    }
    Ok(())
}

/// Write inlines as plain text inside an already-open w:r
fn write_inline_plain(
    w: &mut Writer<impl io::Write>,
    element: &InlineElement,
) -> Result<(), Error> {
    let text = inline_text_content(element);
    w.write_event(Event::Start(
        BytesStart::new("w:t").with_attributes([("xml:space", "preserve")]),
    ))?;
    w.write_event(Event::Text(BytesText::new(&text)))?;
    w.write_event(Event::End(BytesEnd::new("w:t")))?;
    Ok(())
}

fn inline_text_content(element: &InlineElement) -> String {
    match element {
        InlineElement::Text(t) => t.clone(),
        InlineElement::Emphasis(em) => {
            em.content.iter().map(inline_text_content).collect()
        }
        InlineElement::Subscript(c) | InlineElement::Superscript(c) => {
            c.iter().map(inline_text_content).collect()
        }
        InlineElement::Link(link) => {
            link.content.iter().map(inline_text_content).collect()
        }
        InlineElement::InlineCode(c) => c.clone(),
        InlineElement::Citation(k) => format!("[{}]", k),
        InlineElement::CrossReference(l) => format!("§{}", l),
        InlineElement::FootnoteReference(id) => format!("[^{}]", id),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_run_with_text(w: &mut Writer<impl io::Write>, text: &str) {
    let _ = w.write_event(Event::Start(BytesStart::new("w:r")));
    let _ = w.write_event(Event::Start(
        BytesStart::new("w:t").with_attributes([("xml:space", "preserve")]),
    ));
    let _ = w.write_event(Event::Text(BytesText::new(text)));
    let _ = w.write_event(Event::End(BytesEnd::new("w:t")));
    let _ = w.write_event(Event::End(BytesEnd::new("w:r")));
}

fn write_run_empty(w: &mut Writer<impl io::Write>, text: &str) {
    let _ = w.write_event(Event::Start(BytesStart::new("w:r")));
    let _ = w.write_event(Event::Start(
        BytesStart::new("w:t").with_attributes([("xml:space", "preserve")]),
    ));
    let _ = w.write_event(Event::Text(BytesText::new(text)));
    let _ = w.write_event(Event::End(BytesEnd::new("w:t")));
    let _ = w.write_event(Event::End(BytesEnd::new("w:r")));
}

fn write_run_with_formatting(
    w: &mut Writer<impl io::Write>,
    content: &[InlineElement],
    bold: bool,
    italic: bool,
    superscript: bool,
    subscript: bool,
) -> Result<(), Error> {
    w.write_event(Event::Start(BytesStart::new("w:r")))?;
    w.write_event(Event::Start(BytesStart::new("w:rPr")))?;
    if bold {
        w.write_event(Event::Empty(BytesStart::new("w:b")))?;
    }
    if italic {
        w.write_event(Event::Empty(BytesStart::new("w:i")))?;
    }
    if superscript {
        w.write_event(Event::Empty(
            BytesStart::new("w:vertAlign").with_attributes([("w:val", "superscript")]),
        ))?;
    }
    if subscript {
        w.write_event(Event::Empty(
            BytesStart::new("w:vertAlign").with_attributes([("w:val", "subscript")]),
        ))?;
    }
    w.write_event(Event::End(BytesEnd::new("w:rPr")))?;

    for child in content {
        write_inline_plain(w, child)?;
    }

    w.write_event(Event::End(BytesEnd::new("w:r")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;
    use std::io::Cursor;
    use zip::ZipArchive;

    fn make_doc(document: &Document) -> Vec<u8> {
        let backend = DocxBackend;
        let mut buf = Cursor::new(Vec::new());
        backend.compile(document, &mut buf).unwrap();
        buf.into_inner()
    }

    #[test]
    fn test_empty_document() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        // Verify all required files exist
        assert!(zip.by_name("[Content_Types].xml").is_ok());
        assert!(zip.by_name("_rels/.rels").is_ok());
        assert!(zip.by_name("docProps/core.xml").is_ok());
        assert!(zip.by_name("docProps/app.xml").is_ok());
        assert!(zip.by_name("word/styles.xml").is_ok());
        assert!(zip.by_name("word/settings.xml").is_ok());
        assert!(zip.by_name("word/document.xml").is_ok());
        assert!(zip.by_name("word/_rels/document.xml.rels").is_ok());

        // Verify document.xml contains expected OOXML elements
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };
        assert!(doc_xml.contains("w:document"));
        assert!(doc_xml.contains("w:body"));
    }

    #[test]
    fn test_simple_paragraph() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Paragraph(Paragraph {
                content: vec![InlineElement::Text("Hello, world!".to_string())],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };

        assert!(doc_xml.contains("Hello, world!"));
        assert!(doc_xml.contains("w:p"));
        assert!(doc_xml.contains("w:r"));
        assert!(doc_xml.contains("w:t"));
    }

    #[test]
    fn test_heading() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Heading(Heading {
                level: 1,
                content: vec![InlineElement::Text("Introduction".to_string())],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };

        assert!(doc_xml.contains("Introduction"));
        assert!(doc_xml.contains("Heading1"));
    }

    #[test]
    fn test_emphasis_levels() {
        // level 1 = italic, level 2 = bold, level 3 = bold+italic
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![
                BlockElement::Paragraph(Paragraph {
                    content: vec![
                        InlineElement::Emphasis(Emphasis {
                            level: 1,
                            content: vec![InlineElement::Text("italic".to_string())],
                        }),
                    ],
                }),
                BlockElement::Paragraph(Paragraph {
                    content: vec![
                        InlineElement::Emphasis(Emphasis {
                            level: 2,
                            content: vec![InlineElement::Text("bold".to_string())],
                        }),
                    ],
                }),
                BlockElement::Paragraph(Paragraph {
                    content: vec![
                        InlineElement::Emphasis(Emphasis {
                            level: 3,
                            content: vec![InlineElement::Text("both".to_string())],
                        }),
                    ],
                }),
            ],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };

        assert!(doc_xml.contains("italic"));
        assert!(doc_xml.contains("bold"));
        assert!(doc_xml.contains("both"));
        assert!(doc_xml.contains("w:i")); // italic element present
        assert!(doc_xml.contains("w:b")); // bold element present
    }

    #[test]
    fn test_table() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Table(Table {
                caption: None,
                header: vec![Row {
                    cells: vec![
                        Cell {
                            content: vec![BlockElement::Paragraph(Paragraph {
                                content: vec![InlineElement::Text("A".to_string())],
                            })],
                            ..Default::default()
                        },
                        Cell {
                            content: vec![BlockElement::Paragraph(Paragraph {
                                content: vec![InlineElement::Text("B".to_string())],
                            })],
                            ..Default::default()
                        },
                    ],
                }],
                body: vec![Row {
                    cells: vec![
                        Cell {
                            content: vec![BlockElement::Paragraph(Paragraph {
                                content: vec![InlineElement::Text("1".to_string())],
                            })],
                            ..Default::default()
                        },
                        Cell {
                            content: vec![BlockElement::Paragraph(Paragraph {
                                content: vec![InlineElement::Text("2".to_string())],
                            })],
                            ..Default::default()
                        },
                    ],
                }],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };

        assert!(doc_xml.contains("w:tbl"));
        assert!(doc_xml.contains("w:tr"));
        assert!(doc_xml.contains("w:tc"));
        assert!(doc_xml.contains(">A<"));
        assert!(doc_xml.contains(">B<"));
        assert!(doc_xml.contains(">1<"));
        assert!(doc_xml.contains(">2<"));
    }

    #[test]
    fn test_subscript_superscript() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Paragraph(Paragraph {
                content: vec![
                    InlineElement::Text("H".to_string()),
                    InlineElement::Subscript(vec![InlineElement::Text("2".to_string())]),
                    InlineElement::Text("O = mc".to_string()),
                    InlineElement::Superscript(vec![InlineElement::Text("2".to_string())]),
                ],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };

        assert!(doc_xml.contains("subscript"));
        assert!(doc_xml.contains("superscript"));
    }

    #[test]
    fn test_list() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::List(List {
                ordered: false,
                items: vec![
                    ListItem {
                        content: vec![BlockElement::Paragraph(Paragraph {
                            content: vec![InlineElement::Text("first".to_string())],
                        })],
                    },
                    ListItem {
                        content: vec![BlockElement::Paragraph(Paragraph {
                            content: vec![InlineElement::Text("second".to_string())],
                        })],
                    },
                ],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };

        assert!(doc_xml.contains("first"));
        assert!(doc_xml.contains("second"));
        // Bullet character (U+2022) encoded as UTF-8
        assert!(doc_xml.contains("\u{2022}"));
    }

    #[test]
    fn test_inline_code() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Paragraph(Paragraph {
                content: vec![InlineElement::InlineCode("let x = 1;".to_string())],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_doc(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let doc_xml = {
            let mut f = zip.by_name("word/document.xml").unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };

        assert!(doc_xml.contains("let x = 1;"));
        assert!(doc_xml.contains("Consolas"));
    }
}
