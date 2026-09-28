#let data = json(bytes(sys.inputs.data))
#let contact = data.contact
#let cv = data.cv

#set document(title: contact.name + " — CV")
#set page(paper: "a4", margin: (x: 1.6cm, y: 1.4cm))
#set text(font: "Libertinus Serif", size: 10.5pt, lang: "en")
#set par(justify: false, leading: 0.55em)
#show link: set text(fill: rgb("#1f3a93"))

#let section(title) = {
  v(0.7em)
  text(size: 9pt, weight: "bold", tracking: 0.08em, upper(title))
  v(-0.55em)
  line(length: 100%, stroke: 0.4pt + luma(170))
  v(0.1em)
}

#align(left)[
  #text(size: 20pt, weight: "bold", contact.name) \
  #v(-0.3em)
  #text(size: 11pt, cv.headline) \
  #v(-0.2em)
  #text(size: 9pt, fill: luma(80))[
    #(
      (contact.email, contact.phone, contact.location).filter(x => x != "")
        + contact.links.map(l => link(l, l.replace("https://", "")))
    ).join("  ·  ")
  ]
]

#if cv.summary != "" [
  #section("Summary")
  #cv.summary
]

#if cv.skills.len() > 0 [
  #section("Skills")
  #for group in cv.skills [
    *#group.label:* #group.items.join(", ") \
  ]
]

#if cv.experience.len() > 0 [
  #section("Experience")
  #for role in cv.experience [
    #grid(
      columns: (1fr, auto),
      [*#role.title*, #role.company],
      text(fill: luma(80), role.dates),
    )
    #if role.location != "" [ #v(-0.4em) #text(size: 9pt, fill: luma(90), role.location) ]
    #for bullet in role.bullets [ - #bullet ]
    #v(0.3em)
  ]
]

#if cv.projects.len() > 0 [
  #section("Projects")
  #for project in cv.projects [
    *#project.name* #if project.stack.len() > 0 [ #text(size: 9pt, fill: luma(80))[— #project.stack.join(", ")] ]
    #for bullet in project.bullets [ - #bullet ]
    #v(0.2em)
  ]
]

#if cv.education.len() > 0 [
  #section("Education")
  #for school in cv.education [
    #grid(columns: (1fr, auto), [*#school.degree*, #school.school], text(fill: luma(80), school.dates))
  ]
]
