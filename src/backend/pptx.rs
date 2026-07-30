use crate::backend::{Backend, Error};
use crate::ir::{
    BlockElement, Document, InlineElement, Metadata, Row, Table,
};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::{self, Write};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub struct PptxBackend;

impl Backend for PptxBackend {
    fn compile<W: Write + io::Seek>(&self, document: &Document, writer: W) -> Result<(), Error> {
        let mut zip = ZipWriter::new(writer);
        let options = SimpleFileOptions::default();

        let slides: Vec<&BlockElement> = document
            .body
            .iter()
            .filter(|b| matches!(b, BlockElement::Slide(_)))
            .collect();

        let slide_count = slides.len().max(1);

        let has_notes: Vec<bool> = slides
            .iter()
            .map(|b| {
                if let BlockElement::Slide(s) = b {
                    s.notes.is_some()
                } else {
                    false
                }
            })
            .collect();

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(pptx_content_types(slide_count, &has_notes).as_bytes())?;

        // _rels/.rels
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(pptx_rels().as_bytes())?;

        // docProps/core.xml
        zip.start_file("docProps/core.xml", options)?;
        zip.write_all(core_xml(&document.metadata).as_bytes())?;

        // docProps/app.xml
        zip.start_file("docProps/app.xml", options)?;
        zip.write_all(app_xml().as_bytes())?;

        // ppt/presentation.xml
        let mut pres_xml = Vec::new();
        {
            let cursor = io::Cursor::new(&mut pres_xml);
            write_presentation_xml(slide_count, cursor)?;
        }
        zip.start_file("ppt/presentation.xml", options)?;
        zip.write_all(&pres_xml)?;

        // ppt/_rels/presentation.xml.rels
        zip.start_file("ppt/_rels/presentation.xml.rels", options)?;
        zip.write_all(presentation_rels(slide_count).as_bytes())?;

        // ppt/slideMasters/slideMaster1.xml
        zip.start_file("ppt/slideMasters/slideMaster1.xml", options)?;
        zip.write_all(slide_master_xml().as_bytes())?;

        // ppt/slideLayouts/slideLayout1.xml
        zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)?;
        zip.write_all(slide_layout_xml().as_bytes())?;

        // ppt/slideMasters/_rels/slideMaster1.xml.rels
        zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)?;
        zip.write_all(slide_master_rels().as_bytes())?;

        // Write each slide
        for (i, block) in slides.iter().enumerate() {
            let slide_num = i + 1;
            let mut sld = Vec::new();
            {
                let cursor = io::Cursor::new(&mut sld);
                if let BlockElement::Slide(slide) = block {
                    write_slide_xml(slide, cursor)?;
                }
            }
            let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
            zip.start_file(&slide_path, options)?;
            zip.write_all(&sld)?;

            // Slide relationship
            let slide_has_notes = has_notes.get(i).copied().unwrap_or(false);
            let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
            zip.start_file(&rels_path, options)?;
            zip.write_all(slide_rels(slide_num, slide_has_notes).as_bytes())?;

            // Notes slide if the slide has notes
            if let BlockElement::Slide(slide) = block {
                if slide.notes.is_some() {
                    let mut notes_xml = Vec::new();
                    {
                        let cursor = io::Cursor::new(&mut notes_xml);
                        write_notes_slide(slide, cursor)?;
                    }
                    let notes_path = format!("ppt/notesSlides/notesSlide{}.xml", slide_num);
                    zip.start_file(&notes_path, options)?;
                    zip.write_all(&notes_xml)?;

                    let notes_rels_path =
                        format!("ppt/notesSlides/_rels/notesSlide{}.xml.rels", slide_num);
                    zip.start_file(&notes_rels_path, options)?;
                    zip.write_all(notes_slide_rels(slide_num).as_bytes())?;
                }
            }
        }

        // If no slides in body, generate a title slide from metadata
        if slides.is_empty() {
            let mut sld = Vec::new();
            {
                let cursor = io::Cursor::new(&mut sld);
                write_title_slide(&document.metadata, cursor)?;
            }
            zip.start_file("ppt/slides/slide1.xml", options)?;
            zip.write_all(&sld)?;

            zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)?;
            zip.write_all(slide_rels(1, false).as_bytes())?;
        }

        // ppt/tableStyles.xml (required by some validators)
        zip.start_file("ppt/tableStyles.xml", options)?;
        zip.write_all(table_styles_xml().as_bytes())?;

        zip.finish()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Static XML fragments
// ---------------------------------------------------------------------------

fn pptx_content_types(slide_count: usize, has_notes: &[bool]) -> String {
    let mut types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>"#,
    );

    for i in 1..=slide_count {
        types.push_str(&format!(
            "\n  <Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>",
            i
        ));
        if has_notes.get(i - 1).copied().unwrap_or(false) {
            types.push_str(&format!(
                "\n  <Override PartName=\"/ppt/notesSlides/notesSlide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>",
                i
            ));
        }
    }

    types.push_str("\n</Types>");
    types
}

fn pptx_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#.to_string()
}

fn presentation_rels(slide_count: usize) -> String {
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#,
    );

    for i in 1..=slide_count {
        rels.push_str(&format!(
            "\n  <Relationship Id=\"rIds{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>",
            i, i
        ));
    }

    rels.push_str("\n</Relationships>");
    rels
}

fn slide_rels(slide_num: usize, has_notes: bool) -> String {
    if has_notes {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide" Target="../notesSlides/notesSlide{}.xml"/>
</Relationships>"#,
            slide_num
        )
    } else {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#.to_string()
    }
}

fn notes_slide_rels(slide_num: usize) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="../slides/slide{}.xml"/>
</Relationships>"#,
        slide_num
    )
}

fn slide_master_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#.to_string()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn core_xml(metadata: &Metadata) -> String {
    let title = xml_escape(metadata.title.as_deref().unwrap_or(""));
    let author = xml_escape(metadata.author.as_deref().unwrap_or(""));
    let date = xml_escape(metadata.date.as_deref().unwrap_or(""));
    let language = xml_escape(metadata.language.as_deref().unwrap_or(""));

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

fn table_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#.to_string()
}

// ---------------------------------------------------------------------------
// Presentation XML
// ---------------------------------------------------------------------------

fn write_presentation_xml(slide_count: usize, writer: impl io::Write) -> Result<(), Error> {
    let mut w = Writer::new_with_indent(writer, b' ', 2);

    let xmlns_p = "http://schemas.openxmlformats.org/presentationml/2006/main";
    let xmlns_a = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let xmlns_r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;
    w.write_event(Event::Start(
        BytesStart::new("p:presentation")
            .with_attributes([("xmlns:p", xmlns_p), ("xmlns:a", xmlns_a), ("xmlns:r", xmlns_r)]),
    ))?;

    // Slide size (16:9 widescreen)
    w.write_event(Event::Empty(
        BytesStart::new("p:sldSz")
            .with_attributes([("cx", "12192000"), ("cy", "6858000")]),
    ))?;

    // Slide ID list
    w.write_event(Event::Start(BytesStart::new("p:sldIdLst")))?;
    for i in 1..=slide_count {
        let id_str = (i * 256).to_string();
        let r_id = format!("rIds{}", i);
        w.write_event(Event::Empty(
            BytesStart::new("p:sldId")
                .with_attributes([("id", id_str.as_str()), ("r:id", r_id.as_str())]),
        ))?;
    }
    w.write_event(Event::End(BytesEnd::new("p:sldIdLst")))?;

    w.write_event(Event::End(BytesEnd::new("p:presentation")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Slide Master
// ---------------------------------------------------------------------------

fn slide_master_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill>
          <a:srgbClr val="FFFFFF"/>
        </a:solidFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Title Placeholder"/>
          <p:cNvSpPr>
            <a:spLocks noGrp="1"/>
          </p:cNvSpPr>
          <p:nvPr>
            <p:ph type="title"/>
          </p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="685800" y="274320"/>
            <a:ext cx="10820400" cy="1143000"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US" sz="4400">
                <a:latin typeface="Calibri"/>
              </a:rPr>
              <a:t>Title</a:t>
            </a:r>
            <a:endParaRPr lang="en-US" sz="4400">
              <a:latin typeface="Calibri"/>
            </a:endParaRPr>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Body Placeholder"/>
          <p:cNvSpPr>
            <a:spLocks noGrp="1"/>
          </p:cNvSpPr>
          <p:nvPr>
            <p:ph type="body" idx="1"/>
          </p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="685800" y="1600200"/>
            <a:ext cx="10820400" cy="5029200"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang="en-US" sz="2800">
                <a:latin typeface="Calibri"/>
              </a:rPr>
              <a:t>Content</a:t>
            </a:r>
            <a:endParaRPr lang="en-US" sz="2800">
              <a:latin typeface="Calibri"/>
            </a:endParaRPr>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldMaster>"#.to_string()
}

fn slide_layout_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             type="title">
  <p:cSld name="Title Slide">
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Title"/>
          <p:cNvSpPr>
            <a:spLocks noGrp="1"/>
          </p:cNvSpPr>
          <p:nvPr>
            <p:ph type="title"/>
          </p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="685800" y="274320"/>
            <a:ext cx="10820400" cy="1143000"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:endParaRPr lang="en-US" sz="4400">
              <a:latin typeface="Calibri"/>
            </a:endParaRPr>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Body"/>
          <p:cNvSpPr>
            <a:spLocks noGrp="1"/>
          </p:cNvSpPr>
          <p:nvPr>
            <p:ph type="body" idx="1"/>
          </p:nvPr>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm>
            <a:off x="685800" y="1600200"/>
            <a:ext cx="10820400" cy="5029200"/>
          </a:xfrm>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:endParaRPr lang="en-US" sz="2800">
              <a:latin typeface="Calibri"/>
            </a:endParaRPr>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sldLayout>"#.to_string()
}

// ---------------------------------------------------------------------------
// Slide XML
// ---------------------------------------------------------------------------

fn write_slide_xml(slide: &crate::ir::Slide, writer: impl io::Write) -> Result<(), Error> {
    let mut w = Writer::new_with_indent(writer, b' ', 2);

    let xmlns_p = "http://schemas.openxmlformats.org/presentationml/2006/main";
    let xmlns_a = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let xmlns_r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;
    w.write_event(Event::Start(
        BytesStart::new("p:sld")
            .with_attributes([("xmlns:p", xmlns_p), ("xmlns:a", xmlns_a), ("xmlns:r", xmlns_r)]),
    ))?;

    w.write_event(Event::Start(BytesStart::new("p:cSld")))?;

    // Background
    w.write_event(Event::Start(BytesStart::new("p:bg")))?;
    w.write_event(Event::Start(BytesStart::new("p:bgPr")))?;
    w.write_event(Event::Start(BytesStart::new("a:solidFill")))?;
    w.write_event(Event::Empty(BytesStart::new("a:srgbClr").with_attributes([("val", "FFFFFF")])))?;
    w.write_event(Event::End(BytesEnd::new("a:solidFill")))?;
    w.write_event(Event::End(BytesEnd::new("p:bgPr")))?;
    w.write_event(Event::End(BytesEnd::new("p:bg")))?;

    // Shape tree
    w.write_event(Event::Start(BytesStart::new("p:spTree")))?;

    // Title shape
    let title_text = inlines_to_text(&slide.title);
    let shape_id = 1u32;
    write_slide_title(&mut w, shape_id, &title_text)?;

    // Content shapes
    let mut next_id = shape_id + 1;
    for block in &slide.content {
        next_id = write_slide_block(&mut w, block, next_id)?;
    }

    w.write_event(Event::End(BytesEnd::new("p:spTree")))?;
    w.write_event(Event::End(BytesEnd::new("p:cSld")))?;
    w.write_event(Event::End(BytesEnd::new("p:sld")))?;

    Ok(())
}

fn write_title_slide(metadata: &Metadata, writer: impl io::Write) -> Result<(), Error> {
    let mut w = Writer::new_with_indent(writer, b' ', 2);

    let xmlns_p = "http://schemas.openxmlformats.org/presentationml/2006/main";
    let xmlns_a = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let xmlns_r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;
    w.write_event(Event::Start(
        BytesStart::new("p:sld")
            .with_attributes([("xmlns:p", xmlns_p), ("xmlns:a", xmlns_a), ("xmlns:r", xmlns_r)]),
    ))?;

    w.write_event(Event::Start(BytesStart::new("p:cSld")))?;

    // Background
    w.write_event(Event::Start(BytesStart::new("p:bg")))?;
    w.write_event(Event::Start(BytesStart::new("p:bgPr")))?;
    w.write_event(Event::Start(BytesStart::new("a:solidFill")))?;
    w.write_event(Event::Empty(BytesStart::new("a:srgbClr").with_attributes([("val", "FFFFFF")])))?;
    w.write_event(Event::End(BytesEnd::new("a:solidFill")))?;
    w.write_event(Event::End(BytesEnd::new("p:bgPr")))?;
    w.write_event(Event::End(BytesEnd::new("p:bg")))?;

    w.write_event(Event::Start(BytesStart::new("p:spTree")))?;

    let title = metadata.title.as_deref().unwrap_or("Untitled");
    let author = metadata.author.as_deref().unwrap_or("");

    write_slide_title(&mut w, 1, title)?;

    if !author.is_empty() {
        write_text_box(&mut w, 2, 685800, 1600200, 10820400, 600000, author, 2400)?;
    }

    w.write_event(Event::End(BytesEnd::new("p:spTree")))?;
    w.write_event(Event::End(BytesEnd::new("p:cSld")))?;
    w.write_event(Event::End(BytesEnd::new("p:sld")))?;

    Ok(())
}

fn write_slide_title(
    w: &mut Writer<impl io::Write>,
    id: u32,
    title: &str,
) -> Result<(), Error> {
    let id_str = id.to_string();

    w.write_event(Event::Start(BytesStart::new("p:sp")))?;

    // Non-visual properties
    w.write_event(Event::Start(BytesStart::new("p:nvSpPr")))?;
    let name = format!("Title {}", id);
    w.write_event(Event::Empty(
        BytesStart::new("p:cNvPr")
            .with_attributes([("id", id_str.as_str()), ("name", name.as_str())]),
    ))?;
    w.write_event(Event::Start(BytesStart::new("p:cNvSpPr")))?;
    w.write_event(Event::Empty(BytesStart::new("a:spLocks").with_attributes([("noGrp", "1")])))?;
    w.write_event(Event::End(BytesEnd::new("p:cNvSpPr")))?;
    w.write_event(Event::Start(BytesStart::new("p:nvPr")))?;
    w.write_event(Event::Empty(BytesStart::new("p:ph").with_attributes([("type", "title")])))?;
    w.write_event(Event::End(BytesEnd::new("p:nvPr")))?;
    w.write_event(Event::End(BytesEnd::new("p:nvSpPr")))?;

    // Shape properties
    w.write_event(Event::Start(BytesStart::new("p:spPr")))?;
    w.write_event(Event::Start(BytesStart::new("a:xfrm")))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:off").with_attributes([("x", "685800"), ("y", "274320")]),
    ))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:ext").with_attributes([("cx", "10820400"), ("cy", "1143000")]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("a:xfrm")))?;
    w.write_event(Event::End(BytesEnd::new("p:spPr")))?;

    // Text body
    w.write_event(Event::Start(BytesStart::new("p:txBody")))?;
    w.write_event(Event::Start(BytesStart::new("a:bodyPr")))?;
    w.write_event(Event::End(BytesEnd::new("a:bodyPr")))?;
    w.write_event(Event::Empty(BytesStart::new("a:lstStyle")))?;

    w.write_event(Event::Start(BytesStart::new("a:p")))?;
    w.write_event(Event::Start(BytesStart::new("a:r")))?;
    w.write_event(Event::Start(
        BytesStart::new("a:rPr")
            .with_attributes([("lang", "en-US"), ("sz", "4400")]),
    ))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:latin").with_attributes([("typeface", "Calibri")]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("a:rPr")))?;
    w.write_event(Event::Start(BytesStart::new("a:t")))?;
    w.write_event(Event::Text(BytesText::new(title)))?;
    w.write_event(Event::End(BytesEnd::new("a:t")))?;
    w.write_event(Event::End(BytesEnd::new("a:r")))?;
    // End paragraph run properties
    w.write_event(Event::Start(BytesStart::new("a:endParaRPr").with_attributes([
        ("lang", "en-US"),
        ("sz", "4400"),
    ])))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:latin").with_attributes([("typeface", "Calibri")]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("a:endParaRPr")))?;
    w.write_event(Event::End(BytesEnd::new("a:p")))?;

    w.write_event(Event::End(BytesEnd::new("p:txBody")))?;
    w.write_event(Event::End(BytesEnd::new("p:sp")))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_text_box(
    w: &mut Writer<impl io::Write>,
    id: u32,
    x: i64,
    y: i64,
    cx: i64,
    cy: i64,
    text: &str,
    font_size: i64,
) -> Result<(), Error> {
    let id_str = id.to_string();
    let x_str = x.to_string();
    let y_str = y.to_string();
    let cx_str = cx.to_string();
    let cy_str = cy.to_string();
    let sz_str = font_size.to_string();

    w.write_event(Event::Start(BytesStart::new("p:sp")))?;

    w.write_event(Event::Start(BytesStart::new("p:nvSpPr")))?;
    let name = format!("TextBox {}", id);
    w.write_event(Event::Empty(
        BytesStart::new("p:cNvPr")
            .with_attributes([("id", id_str.as_str()), ("name", name.as_str())]),
    ))?;
    w.write_event(Event::Start(BytesStart::new("p:cNvSpPr")))?;
    w.write_event(Event::Empty(BytesStart::new("a:spLocks").with_attributes([("noGrp", "1")])))?;
    w.write_event(Event::End(BytesEnd::new("p:cNvSpPr")))?;
    w.write_event(Event::Start(BytesStart::new("p:nvPr")))?;
    w.write_event(Event::End(BytesEnd::new("p:nvPr")))?;
    w.write_event(Event::End(BytesEnd::new("p:nvSpPr")))?;

    w.write_event(Event::Start(BytesStart::new("p:spPr")))?;
    w.write_event(Event::Start(BytesStart::new("a:xfrm")))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:off")
            .with_attributes([("x", x_str.as_str()), ("y", y_str.as_str())]),
    ))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:ext")
            .with_attributes([("cx", cx_str.as_str()), ("cy", cy_str.as_str())]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("a:xfrm")))?;
    w.write_event(Event::End(BytesEnd::new("p:spPr")))?;

    w.write_event(Event::Start(BytesStart::new("p:txBody")))?;
    w.write_event(Event::Start(BytesStart::new("a:bodyPr")))?;
    w.write_event(Event::End(BytesEnd::new("a:bodyPr")))?;
    w.write_event(Event::Empty(BytesStart::new("a:lstStyle")))?;

    w.write_event(Event::Start(BytesStart::new("a:p")))?;
    w.write_event(Event::Start(BytesStart::new("a:r")))?;
    w.write_event(Event::Start(
        BytesStart::new("a:rPr").with_attributes([("lang", "en-US"), ("sz", sz_str.as_str())]),
    ))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:latin").with_attributes([("typeface", "Calibri")]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("a:rPr")))?;
    w.write_event(Event::Start(BytesStart::new("a:t")))?;
    w.write_event(Event::Text(BytesText::new(text)))?;
    w.write_event(Event::End(BytesEnd::new("a:t")))?;
    w.write_event(Event::End(BytesEnd::new("a:r")))?;
    w.write_event(Event::Start(
        BytesStart::new("a:endParaRPr").with_attributes([("lang", "en-US"), ("sz", sz_str.as_str())]),
    ))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:latin").with_attributes([("typeface", "Calibri")]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("a:endParaRPr")))?;
    w.write_event(Event::End(BytesEnd::new("a:p")))?;

    w.write_event(Event::End(BytesEnd::new("p:txBody")))?;
    w.write_event(Event::End(BytesEnd::new("p:sp")))?;

    Ok(())
}

/// Write a block element as a slide shape. Returns the next available shape ID.
fn write_slide_block(
    w: &mut Writer<impl io::Write>,
    block: &BlockElement,
    id: u32,
) -> Result<u32, Error> {
    match block {
        BlockElement::Paragraph(p) => {
            let text = inlines_to_text(&p.content);
            // Compute vertical position based on a simple counter — each paragraph
            // gets 600000 EMU (about 0.5 inches) of height
            let y = 1600200i64 + (id as i64 - 2) * 600000;
            write_text_box(w, id, 685800, y, 10820400, 500000, &text, 2400)?;
            Ok(id + 1)
        }
        BlockElement::List(list) => {
            let mut next_id = id;
            for (i, item) in list.items.iter().enumerate() {
                let mut text = String::new();
                for content_block in &item.content {
                    if let BlockElement::Paragraph(p) = content_block {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        if list.ordered {
                            text.push_str(&format!("{}. ", i + 1));
                        } else {
                            text.push('\u{2022}');
                            text.push(' ');
                        }
                        text.push_str(&inlines_to_text(&p.content));
                    }
                }
                let y = 1600200i64 + (next_id as i64 - 2) * 600000;
                write_text_box(w, next_id, 685800, y, 10820400, 500000, &text, 2400)?;
                next_id += 1;
            }
            Ok(next_id)
        }
        BlockElement::Heading(h) => {
            let text = inlines_to_text(&h.content);
            let y = 1600200i64 + (id as i64 - 2) * 600000;
            let font_size = match h.level {
                1 => 3600,
                2 => 3000,
                _ => 2400,
            };
            write_text_box(w, id, 685800, y, 10820400, 500000, &text, font_size)?;
            Ok(id + 1)
        }
        BlockElement::CodeBlock(cb) => {
            let y = 1600200i64 + (id as i64 - 2) * 600000;
            write_text_box(w, id, 685800, y, 10820400, 500000, &cb.content, 2000)?;
            Ok(id + 1)
        }
        BlockElement::Table(tbl) => {
            let next_id = write_slide_table(w, tbl, id)?;
            Ok(next_id)
        }
        // Skip elements that don't make sense on slides
        BlockElement::TableOfContents(_)
        | BlockElement::Slide(_)
        | BlockElement::BlockQuote(_)
        | BlockElement::Figure(_) => Ok(id),
    }
}

fn write_slide_table(
    w: &mut Writer<impl io::Write>,
    table: &Table,
    id: u32,
) -> Result<u32, Error> {
    let xmlns_a = "http://schemas.openxmlformats.org/drawingml/2006/main";

    let num_cols = table
        .header
        .first()
        .or(table.body.first())
        .map(|r| r.cells.len())
        .unwrap_or(1)
        .max(1);

    let id_str = id.to_string();
    let y = 1600200i64 + (id as i64 - 2) * 600000;

    w.write_event(Event::Start(BytesStart::new("p:graphicFrame")))?;

    // Non-visual properties
    w.write_event(Event::Start(BytesStart::new("p:nvGraphicFramePr")))?;
    let name = format!("Table {}", id);
    w.write_event(Event::Empty(
        BytesStart::new("p:cNvPr")
            .with_attributes([("id", id_str.as_str()), ("name", name.as_str())]),
    ))?;
    w.write_event(Event::Start(BytesStart::new("p:cNvGraphicFramePr")))?;
    w.write_event(Event::Empty(
        BytesStart::new("a:graphicFrameLocks").with_attributes([("noGrp", "1")]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("p:cNvGraphicFramePr")))?;
    w.write_event(Event::Start(BytesStart::new("p:nvPr")))?;
    w.write_event(Event::End(BytesEnd::new("p:nvPr")))?;
    w.write_event(Event::End(BytesEnd::new("p:nvGraphicFramePr")))?;

    // Transform
    w.write_event(Event::Start(BytesStart::new("p:xfrm")))?;
    let y_str = y.to_string();
    w.write_event(Event::Empty(
        BytesStart::new("a:off").with_attributes([("x", "685800"), ("y", y_str.as_str())]),
    ))?;
    let table_width = (num_cols as i64 * 2000000).min(10820400);
    let tw_str = table_width.to_string();
    w.write_event(Event::Empty(
        BytesStart::new("a:ext")
            .with_attributes([("cx", tw_str.as_str()), ("cy", "3000000")]),
    ))?;
    w.write_event(Event::End(BytesEnd::new("p:xfrm")))?;

    // Graphic
    w.write_event(Event::Start(BytesStart::new("a:graphic")))?;
    w.write_event(Event::Start(BytesStart::new("a:graphicData").with_attributes([("uri", xmlns_a)])))?;

    // Table grid
    w.write_event(Event::Start(BytesStart::new("a:tbl")))?;
    w.write_event(Event::Start(BytesStart::new("a:tblPr")))?;
    w.write_event(Event::End(BytesEnd::new("a:tblPr")))?;

    w.write_event(Event::Start(BytesStart::new("a:tblGrid")))?;
    let col_width = (table_width - 100) / num_cols as i64;
    let cw_str = col_width.to_string();
    for _ in 0..num_cols {
        w.write_event(Event::Empty(
            BytesStart::new("a:gridCol").with_attributes([("w", cw_str.as_str())]),
        ))?;
    }
    w.write_event(Event::End(BytesEnd::new("a:tblGrid")))?;

    // Write header rows
    for row in &table.header {
        write_pptx_table_row(w, row, true)?;
    }

    // Write body rows
    for row in &table.body {
        write_pptx_table_row(w, row, false)?;
    }

    w.write_event(Event::End(BytesEnd::new("a:tbl")))?;
    w.write_event(Event::End(BytesEnd::new("a:graphicData")))?;
    w.write_event(Event::End(BytesEnd::new("a:graphic")))?;
    w.write_event(Event::End(BytesEnd::new("p:graphicFrame")))?;

    Ok(id + 1)
}

fn write_pptx_table_row(
    w: &mut Writer<impl io::Write>,
    row: &Row,
    is_header: bool,
) -> Result<(), Error> {
    w.write_event(Event::Start(BytesStart::new("a:tr")))?;

    for cell in &row.cells {
        w.write_event(Event::Start(BytesStart::new("a:tc")))?;

        let text = cell
            .content
            .iter()
            .filter_map(|b| {
                if let BlockElement::Paragraph(p) = b {
                    Some(inlines_to_text(&p.content))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        w.write_event(Event::Start(BytesStart::new("a:txBody")))?;
        w.write_event(Event::Start(BytesStart::new("a:bodyPr")))?;
        w.write_event(Event::End(BytesEnd::new("a:bodyPr")))?;
        w.write_event(Event::Empty(BytesStart::new("a:lstStyle")))?;

        w.write_event(Event::Start(BytesStart::new("a:p")))?;
        w.write_event(Event::Start(BytesStart::new("a:r")))?;
        w.write_event(Event::Start(
            BytesStart::new("a:rPr")
                .with_attributes([("lang", "en-US"), ("sz", if is_header { "2400" } else { "2000" })]),
        ))?;
        if is_header {
            w.write_event(Event::Empty(BytesStart::new("a:b")))?;
        }
        w.write_event(Event::Empty(
            BytesStart::new("a:latin").with_attributes([("typeface", "Calibri")]),
        ))?;
        w.write_event(Event::End(BytesEnd::new("a:rPr")))?;
        w.write_event(Event::Start(BytesStart::new("a:t")))?;
        w.write_event(Event::Text(BytesText::new(&text)))?;
        w.write_event(Event::End(BytesEnd::new("a:t")))?;
        w.write_event(Event::End(BytesEnd::new("a:r")))?;
        w.write_event(Event::Start(
            BytesStart::new("a:endParaRPr").with_attributes([("lang", "en-US"), ("sz", "2000")]),
        ))?;
        w.write_event(Event::Empty(
            BytesStart::new("a:latin").with_attributes([("typeface", "Calibri")]),
        ))?;
        w.write_event(Event::End(BytesEnd::new("a:endParaRPr")))?;
        w.write_event(Event::End(BytesEnd::new("a:p")))?;

        w.write_event(Event::End(BytesEnd::new("a:txBody")))?;

        // Cell properties
        w.write_event(Event::Start(BytesStart::new("a:tcPr")))?;
        if cell.colspan > 1 {
            let cs_str = cell.colspan.to_string();
            w.write_event(Event::Empty(
                BytesStart::new("a:gridSpan").with_attributes([("val", cs_str.as_str())]),
            ))?;
        }
        w.write_event(Event::End(BytesEnd::new("a:tcPr")))?;

        w.write_event(Event::End(BytesEnd::new("a:tc")))?;
    }

    w.write_event(Event::End(BytesEnd::new("a:tr")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Notes Slide
// ---------------------------------------------------------------------------

fn write_notes_slide(slide: &crate::ir::Slide, writer: impl io::Write) -> Result<(), Error> {
    let mut w = Writer::new_with_indent(writer, b' ', 2);

    let xmlns_p = "http://schemas.openxmlformats.org/presentationml/2006/main";
    let xmlns_a = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let xmlns_r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("yes"))))?;
    w.write_event(Event::Start(
        BytesStart::new("p:notes")
            .with_attributes([("xmlns:p", xmlns_p), ("xmlns:a", xmlns_a), ("xmlns:r", xmlns_r)]),
    ))?;

    w.write_event(Event::Start(BytesStart::new("p:cSld")))?;
    w.write_event(Event::Start(BytesStart::new("p:spTree")))?;

    let notes_text = slide
        .notes
        .as_ref()
        .map(|n| inlines_to_text(n))
        .unwrap_or_default();

    write_text_box(&mut w, 1, 685800, 685800, 10820400, 5486400, &notes_text, 2000)?;

    w.write_event(Event::End(BytesEnd::new("p:spTree")))?;
    w.write_event(Event::End(BytesEnd::new("p:cSld")))?;
    w.write_event(Event::End(BytesEnd::new("p:notes")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn inlines_to_text(inlines: &[InlineElement]) -> String {
    inlines.iter().map(inline_text).collect()
}

fn inline_text(element: &InlineElement) -> String {
    match element {
        InlineElement::Text(t) => t.clone(),
        InlineElement::Emphasis(em) => em.content.iter().map(inline_text).collect(),
        InlineElement::Subscript(c) | InlineElement::Superscript(c) => {
            c.iter().map(inline_text).collect()
        }
        InlineElement::Link(link) => link.content.iter().map(inline_text).collect(),
        InlineElement::InlineCode(c) => c.clone(),
        InlineElement::Citation(k) => format!("[{}]", k),
        InlineElement::CrossReference(l) => format!("§{}", l),
        InlineElement::FootnoteReference(id) => format!("[^{}]", id),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;
    use std::io::{Cursor, Read};
    use zip::ZipArchive;

    fn make_pptx(document: &Document) -> Vec<u8> {
        let backend = PptxBackend;
        let mut buf = Cursor::new(Vec::new());
        backend.compile(document, &mut buf).unwrap();
        buf.into_inner()
    }

    #[test]
    fn test_empty_pptx() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        assert!(zip.by_name("[Content_Types].xml").is_ok());
        assert!(zip.by_name("ppt/presentation.xml").is_ok());
        assert!(zip.by_name("ppt/slideMasters/slideMaster1.xml").is_ok());
        assert!(zip.by_name("ppt/slideLayouts/slideLayout1.xml").is_ok());
        // A title slide is always generated
        assert!(zip.by_name("ppt/slides/slide1.xml").is_ok());

        let pres_xml = {
            let mut f = zip.by_name("ppt/presentation.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };
        assert!(pres_xml.contains("p:presentation"));
    }

    #[test]
    fn test_single_slide() {
        let doc = Document {
            metadata: Metadata {
                title: Some("Test Presentation".to_string()),
                ..Default::default()
            },
            body: vec![BlockElement::Slide(Slide {
                title: vec![InlineElement::Text("Slide 1".to_string())],
                notes: None,
                content: vec![BlockElement::Paragraph(Paragraph {
                    content: vec![InlineElement::Text("Hello from PPTX".to_string())],
                })],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        let slide_xml = {
            let mut f = zip.by_name("ppt/slides/slide1.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };

        assert!(slide_xml.contains("Slide 1"));
        assert!(slide_xml.contains("Hello from PPTX"));
        assert!(slide_xml.contains("p:sld"));
        assert!(slide_xml.contains("p:spTree"));
    }

    #[test]
    fn test_slide_with_notes() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Slide(Slide {
                title: vec![InlineElement::Text("Notes test".to_string())],
                notes: Some(vec![InlineElement::Text("Speaker notes here".to_string())]),
                content: vec![],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        assert!(zip.by_name("ppt/notesSlides/notesSlide1.xml").is_ok());

        let notes_xml = {
            let mut f = zip.by_name("ppt/notesSlides/notesSlide1.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };

        assert!(notes_xml.contains("Speaker notes here"));
        assert!(notes_xml.contains("p:notes"));
    }

    #[test]
    fn test_slide_with_list() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Slide(Slide {
                title: vec![InlineElement::Text("List Slide".to_string())],
                notes: None,
                content: vec![BlockElement::List(List {
                    ordered: false,
                    items: vec![
                        ListItem {
                            content: vec![BlockElement::Paragraph(Paragraph {
                                content: vec![InlineElement::Text("Item A".to_string())],
                            })],
                        },
                        ListItem {
                            content: vec![BlockElement::Paragraph(Paragraph {
                                content: vec![InlineElement::Text("Item B".to_string())],
                            })],
                        },
                    ],
                })],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        let slide_xml = {
            let mut f = zip.by_name("ppt/slides/slide1.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };

        assert!(slide_xml.contains("Item A"));
        assert!(slide_xml.contains("Item B"));
        assert!(slide_xml.contains("\u{2022}"));
    }

    #[test]
    fn test_slide_without_notes_has_no_notes_artifacts() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Slide(Slide {
                title: vec![InlineElement::Text("No Notes".to_string())],
                notes: None,
                content: vec![BlockElement::Paragraph(Paragraph {
                    content: vec![InlineElement::Text("Content".to_string())],
                })],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        // Notes slide files should not exist
        assert!(zip.by_name("ppt/notesSlides/notesSlide1.xml").is_err());

        // Slide rels should NOT reference notesSlide
        let rels_xml = {
            let mut f = zip
                .by_name("ppt/slides/_rels/slide1.xml.rels")
                .unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };
        assert!(!rels_xml.contains("notesSlide"));

        // Content types should NOT have notesSlide override
        let ct_xml = {
            let mut f = zip.by_name("[Content_Types].xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };
        assert!(!ct_xml.contains("notesSlide"));
    }

    #[test]
    fn test_multiple_slides() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![
                BlockElement::Slide(Slide {
                    title: vec![InlineElement::Text("Slide 1".to_string())],
                    notes: None,
                    content: vec![BlockElement::Paragraph(Paragraph {
                        content: vec![InlineElement::Text("First".to_string())],
                    })],
                }),
                BlockElement::Slide(Slide {
                    title: vec![InlineElement::Text("Slide 2".to_string())],
                    notes: None,
                    content: vec![BlockElement::Paragraph(Paragraph {
                        content: vec![InlineElement::Text("Second".to_string())],
                    })],
                }),
            ],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        assert!(zip.by_name("ppt/slides/slide1.xml").is_ok());
        assert!(zip.by_name("ppt/slides/slide2.xml").is_ok());

        let pres_xml = {
            let mut f = zip.by_name("ppt/presentation.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };
        assert!(pres_xml.contains("rIds1"));
        assert!(pres_xml.contains("rIds2"));
        // Should NOT reference notesMaster
        assert!(!pres_xml.contains("notesMaster"));
    }

    #[test]
    fn test_mixed_slides_with_and_without_notes() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![
                BlockElement::Slide(Slide {
                    title: vec![InlineElement::Text("With Notes".to_string())],
                    notes: Some(vec![InlineElement::Text("Speaker note".to_string())]),
                    content: vec![],
                }),
                BlockElement::Slide(Slide {
                    title: vec![InlineElement::Text("Without Notes".to_string())],
                    notes: None,
                    content: vec![],
                }),
            ],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        // First slide has notes
        assert!(zip.by_name("ppt/notesSlides/notesSlide1.xml").is_ok());
        // Second slide should NOT have notes
        assert!(zip.by_name("ppt/notesSlides/notesSlide2.xml").is_err());
    }

    #[test]
    fn test_metadata_xml_escaping() {
        let doc = Document {
            metadata: Metadata {
                title: Some("Title <with> & \"chars\"".to_string()),
                ..Default::default()
            },
            body: vec![],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();

        let core_xml = {
            let mut f = zip.by_name("docProps/core.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };

        assert!(core_xml.contains("&lt;with&gt;"));
        assert!(core_xml.contains("&amp; &quot;chars&quot;"));
        assert!(!core_xml.contains("<with>"));
    }

    #[test]
    fn test_slide_with_table() {
        let doc = Document {
            metadata: Metadata::default(),
            body: vec![BlockElement::Slide(Slide {
                title: vec![InlineElement::Text("Table Slide".to_string())],
                notes: None,
                content: vec![BlockElement::Table(Table {
                    caption: None,
                    header: vec![Row {
                        cells: vec![
                            Cell {
                                content: vec![BlockElement::Paragraph(Paragraph {
                                    content: vec![InlineElement::Text("Col1".to_string())],
                                })],
                                ..Default::default()
                            },
                            Cell {
                                content: vec![BlockElement::Paragraph(Paragraph {
                                    content: vec![InlineElement::Text("Col2".to_string())],
                                })],
                                ..Default::default()
                            },
                        ],
                    }],
                    body: vec![Row {
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
                })],
            })],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let slide_xml = {
            let mut f = zip.by_name("ppt/slides/slide1.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };

        assert!(slide_xml.contains("a:tbl"));
        assert!(slide_xml.contains("Col1"));
        assert!(slide_xml.contains("Col2"));
        assert!(slide_xml.contains(">A<"));
        assert!(slide_xml.contains(">B<"));
    }

    #[test]
    fn test_title_slide_when_empty() {
        let doc = Document {
            metadata: Metadata {
                title: Some("My Title".to_string()),
                author: Some("Author Name".to_string()),
                ..Default::default()
            },
            body: vec![],
            footnotes: vec![],
            bibliography: vec![],
        };

        let data = make_pptx(&doc);
        let cursor = Cursor::new(data);
        let mut zip = ZipArchive::new(cursor).unwrap();
        let slide_xml = {
            let mut f = zip.by_name("ppt/slides/slide1.xml").unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            s
        };

        assert!(slide_xml.contains("My Title"));
        assert!(slide_xml.contains("Author Name"));
    }
}
