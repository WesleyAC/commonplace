# Commonplace

Commonplace is a notetaking program. It is a work in progress, I don't recommend that anyone attempt use it right now.

If you're a member of the [Recurse Center](https://www.recurse.com/scout/click?t=288aaf8d6ddfba372520ec10690a1e1b) community, you can follow along with development on Zulip in the [#knowledge systems > wesleyac notes system](https://recurse.zulipchat.com/#narrow/stream/260383-knowledge-systems/topic/wesleyac.20notes.20system) topic, or if you're a member of my personal Zulip community, you can follow along in the [#projects > commonplace](https://wesleyac.zulipchat.com/#narrow/stream/282766-projects/topic/commonplace) topic.

## Philosophy

The fundamental idea behind Commonplace that all others stem from is that the amount of variation in thought processes between different minds is huge, and thus for a tool for thought to be effective, it must be tuned to a particular type of mind. The exact mix of spatial, temporal, visual, kinesthetic, aural, relational, and hierarchical processing capabilities is unique to every mind, and what works for one person is unlikely to work for another.

With that in mind, Commonplace is designed to mesh with my mind, and I do not recommend anyone else try to use it. This is both because it is designed for my mind, and because [situated software](https://web.archive.org/web/20180429085210/http://shirky.com/writings/herecomeseverybody/situated_software.html) offers much faster and more flexible development, since I do not have to cater to any usecases other than my own.

That being said, Commonplace is open-source to allow others to learn from it and copy ideas. Most of the source code is quite poor, but I believe the [SQL schema](/libcommonplace/src/setup.sql) to be quite useful and well thought out, and the [queries](/libcommonplace/src/lib.rs) against it may also be useful.

### How My Brain Works (aka Wesley's Knowledge System Hot Takes)

* Hierarchy is good, as long as you have multiple different hierarchies to play with.
* Crosslinking is mostly bad, since trees are more understandable than graphs.
  * If you really want a graph, a force-directed layout is a terrible way to visualize it, you instead want an embedding that is more consistent, and where things like angular position have actual meaning.
* Notes should be completely unstructured/untagged at first, and only categorized once you've worked with them for a reasonable amount of time.
* "Documents" are better than "blocks" — much of the value of text is in context, and having large documents be built of small blocks allows you to inadvertently strip context from text, destroying a large part of its value.
* The is a lot of value in making your notes a mnemonic medium, by making it seamless to add notes to a spaced repetition system.
* Documents should allow for interactivity by default. "Documents" shouldn't be separate from "programs".
* Deep linking is critically important, especially for mostly-static documents like videos, PDFs, and images.

### Overview

*(Note that this section describes Commonplace as I aspire for it to be, rather than as it is)*

#### Hierarchical Tagging

The key feature of Commonplace is *hierarchical tagging*. There is a tree of tags, and every note can have an unlimited number of tags assigned to it. For instance, a note could be tagged `book` and `programming>languages>rust`, and it would show up in a query for `tag:book tag:programming` or `tag:book tag:programming>languages>rust` (note that querying capabilities have not yet been built, so this query language is currently hypothetical).

One way to use this hierarchical tagging ability to to have multiple different ontologies by which you organize your notes — you can have one hierarchy that organizes by concept (`programming`, `biology`, `philosophy`), another that organizes by medium (`book`, `video`), another that organizes by source (`me`, or the name of a friend or a conference), and another that organizes by time. As Ted Nelson describes, order becomes cumulative, rather than disorder.

Another useful property of hierarchical tagging is that it allows for archiving without destroying existing ontology — simply add the `archive` tag to hide a note from the standard view, while still preserving all existing tags.

The only existing system that I am aware of with a tagging system like this is [Joplin](https://joplinapp.org/).

#### Annotation

An important aspect of a notetaking system is allowing annotation of external works (PDFs, images, websites, videos, etc). This, unfortunately, necessitates linking, and as long as I'm building linking, I may as well build generic linking. However, the UX of linking will be specifically aimed at the usecase of annotation rather than at more generic crosslinking. Deep linking is important for annotation, and needs to be built separately for each filetype (for instance, you should be able to link into videos both temporally and spatially, etc)

#### Mnemonic Medium

It should be easy to take notes and use them as part of a spaced repetition system, to allow you to seamlessly move notes from being remembered by the computer to being remembered by your mind.

### Syncing

It's important to me to be able to see, and ideally edit my notes on my phone. To that end, the data storage format behind Commonplace is designed to be very easy to sync between devices. All notes are stored in a content-addressable storage system, which means that you don't have to worry about overwriting different copies of the files behind a note — you can just add any note to your database without worrying about conflict resolution in the syncing step.

Notes and tags are both referenced by UUID, which avoids conflicts when creating notes or tags on different devices, and merging the tag/note tree is simple — adding tags or notes that were created on a different device never causes an overwrite. (TODO: should tags be reference by UUID? name/path may actually be preferable here, although it does make some things more complicated.)

One cannot escape conflict completely, unfortunately — however, building on top of a content addressable and UUID based system allows for the underlying data synchronization to be trivial, making presentation of conflicts a purely UI concern. Last-write-wins is simple to implement, and more complex merge resolution systems can be added later without changing the fundamental architecture.

# TODOs

* Autosave with timer
* Save note rename on enter press
* Canonicalize tags (if tagged with `misc>foo` and `misc`, only show most specific tag)
* Expand left sidebar tag on right sidebar tag click
* Add "plain"text export
* Support for images
* Support for PDFs
* Add random color to every note
