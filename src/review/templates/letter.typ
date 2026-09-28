#let data = json(bytes(sys.inputs.data))
#let contact = data.contact

#set document(title: contact.name + " — Cover letter, " + data.company)
#set page(paper: "a4", margin: (x: 2.2cm, y: 2cm))
#set text(font: "Libertinus Serif", size: 11pt, lang: "en")
#set par(justify: false, leading: 0.7em, spacing: 1.1em)

#text(size: 16pt, weight: "bold", contact.name) \
#text(size: 9pt, fill: luma(80), ((contact.email, contact.phone, contact.location).filter(x => x != "")).join("  ·  "))

#v(1.2em)
#data.date

#v(0.6em)
#data.letter

#v(0.8em)
#contact.name
