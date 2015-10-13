use core::str::pattern::{DoubleEndedSearcher, Pattern, ReverseSearcher, Searcher};

use utf8_sections::Utf8Sections;

macro_rules! make_iterator {
    ($forward:ident and $reverse:ident wrap $inner:ident yielding $map:expr => $ret:ty) => {
        pub struct $forward<'a, P>($inner<'a, P>) where P: Pattern<'a>;

        impl<'a, P> Clone for $forward<'a, P>
        where P: Pattern<'a> + Clone, P::Searcher: Clone {
            fn clone(&self) -> Self { $forward(self.0.clone()) }
        }

        impl<'a, P> $forward<'a, P> where P: Pattern<'a> {
            pub fn new(slice: &'a [u8], pat: P) -> Self {
                $forward($inner::new(slice, pat))
            }
        }

        impl<'a, P> Iterator for $forward<'a, P> where P: Pattern<'a> + Clone {
            type Item = $ret;

            fn next(&mut self) -> Option<$ret> {
                self.0.next().map($map)
            }
        }

        impl<'a, P> DoubleEndedIterator for $forward<'a, P>
        where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> {
            fn next_back(&mut self) -> Option<$ret> {
                self.0.next_back().map($map)
            }
        }

        pub struct $reverse<'a, P>($inner<'a, P>) where P: Pattern<'a>;

        impl<'a, P> Clone for $reverse<'a, P>
        where P: Pattern<'a> + Clone, P::Searcher: Clone {
            fn clone(&self) -> Self { $reverse(self.0.clone()) }
        }

        impl<'a, P> $reverse<'a, P> where P: Pattern<'a> {
            pub fn new(slice: &'a [u8], pat: P) -> Self {
                $reverse($inner::new(slice, pat))
            }
        }

        impl<'a, P> Iterator for $reverse<'a, P>
        where P: Pattern<'a> + Clone, P::Searcher: ReverseSearcher<'a> {
            type Item = $ret;

            fn next(&mut self) -> Option<$ret> {
                self.0.next_back().map($map)
            }
        }

        impl<'a, P> DoubleEndedIterator for $reverse<'a, P>
        where P: Pattern<'a> + Clone, P::Searcher: DoubleEndedSearcher<'a> {
            fn next_back(&mut self) -> Option<$ret> {
                self.0.next().map($map)
            }
        }
    }
}

make_iterator!{Matches and RMatches wrap MatchImpl yielding |x| x.1 => &'a str}
make_iterator!{Split and RSplit wrap SplitImpl yielding |x| x => &'a [u8]}


struct MatchImpl<'a, P> where P: Pattern<'a> {
    sections: Utf8Sections<'a>,
    pat: P,
    front_searcher: Option<P::Searcher>,
    back_searcher: Option<P::Searcher>,
    front_section: (usize, &'a str),
    back_section: (usize, &'a str),
}

impl<'a, P> Clone for MatchImpl<'a, P>
where P: Pattern<'a> + Clone, P::Searcher: Clone {
    fn clone(&self) -> Self {
        MatchImpl {
            sections: self.sections.clone(),
            pat: self.pat.clone(),
            front_searcher: self.front_searcher.clone(),
            back_searcher: self.back_searcher.clone(),
            front_section: self.front_section.clone(),
            back_section: self.back_section.clone(),
        }
    }
}

impl<'a, P> MatchImpl<'a, P> where P: Pattern<'a> {
    pub fn new(slice: &'a [u8], pat: P) -> Self {
        MatchImpl {
            pat: pat,
            sections: Utf8Sections::new(slice),
            front_searcher: None,
            back_searcher: None,
            front_section: (0, ""),
            back_section: (slice.len(), ""),
        }
    }

    fn next(&mut self) -> Option<(usize, &'a str)> where P: Clone {
        loop {
            if self.front_section.0 == self.back_section.0 {
                // Last section.  Both directions use the same iterator.
                let searcher = self.front_searcher.as_mut().or(self.back_searcher.as_mut());
                let section = self.front_section;
                return searcher.and_then(|m| m.next_match())
                    .map(|(start, end)| (section.0 + start, &section.1[start..end]));
            }
            if self.front_searcher.is_none() {
                if let Some(section) = self.sections.next() {
                    self.front_section = section;
                    // New segment
                    self.front_searcher =
                        Some(self.pat.clone().into_searcher(self.front_section.1));
                } else {
                    // Starting the last segment.  Go back to the top.
                    self.front_section = self.back_section;
                    continue;
                }
            }

            if let Some((start, end)) = self.front_searcher.as_mut().unwrap().next_match() {
                let section = self.front_section;
                return Some((section.0 + start, &section.1[start..end]));
            }

            // Finished this segment
            self.front_searcher = None;
        }
    }

    fn next_back(&mut self) -> Option<(usize, &'a str)>
    where P: Clone, P::Searcher: ReverseSearcher<'a> {
        loop {
            if self.front_section.0 == self.back_section.0 {
                // Last section.  Both directions use the same iterator.
                let searcher = self.front_searcher.as_mut().or(self.back_searcher.as_mut());
                let section = self.back_section;
                return searcher.and_then(|m| m.next_match_back())
                    .map(|(start, end)| (section.0 + start, &section.1[start..end]));
            }
            if self.back_searcher.is_none() {
                if let Some(section) = self.sections.next_back() {
                    // New segment
                    self.back_section = section;
                    self.back_searcher =
                        Some(self.pat.clone().into_searcher(self.back_section.1));
                } else {
                    // Starting the last segment.  Go back to the top.
                    self.back_section = self.front_section;
                    continue;
                }
            }

            if let Some((start, end)) = self.back_searcher.as_mut().unwrap().next_match_back() {
                let section = self.back_section;
                return Some((section.0 + start, &section.1[start..end]));
            }

            // Finished this segment
            self.back_searcher = None;
        }
    }
}

struct SplitImpl<'a, P> where P: Pattern<'a> {
    slice: &'a [u8],
    matches: MatchImpl<'a, P>,
    remainder: (usize, usize),
}

impl<'a, P> Clone for SplitImpl<'a, P>
where P: Pattern<'a> + Clone, P::Searcher: Clone {
    fn clone(&self) -> Self {
        SplitImpl {
            slice: self.slice.clone(),
            matches: self.matches.clone(),
            remainder: self.remainder.clone(),
        }
    }
}

impl<'a, P> SplitImpl<'a, P> where P: Pattern<'a> {
    pub fn new(slice: &'a [u8], pat: P) -> Self {
        SplitImpl {
            slice: slice,
            matches: MatchImpl::new(slice, pat),
            remainder: (0, slice.len()),
        }
    }

    fn next(&mut self) -> Option<&'a [u8]> where P: Clone {
        if self.remainder.1 < self.remainder.0 { return None; }
        if let Some((start, mat)) = self.matches.next() {
            let result = &self.slice[self.remainder.0..start];
            self.remainder.0 = start + mat.len();
            Some(result)
        } else {
            // We've exhausted all the matches, but we still
            // have the section between the last matches from
            // either side to return.
            let result = &self.slice[self.remainder.0..self.remainder.1];
            self.remainder.0 = self.remainder.1 + 1;
            Some(result)
        }
    }

    fn next_back(&mut self) -> Option<&'a [u8]>
    where P: Clone, P::Searcher: ReverseSearcher<'a> {
        if self.remainder.1 < self.remainder.0 { return None; }
        if let Some((start, mat)) = self.matches.next_back() {
            let result = &self.slice[start + mat.len()..self.remainder.1];
            self.remainder.1 = start;
            Some(result)
        } else {
            // We've exhausted all the matches, but we still
            // have the section between the last matches from
            // either side to return.
            let result = &self.slice[self.remainder.0..self.remainder.1];
            self.remainder.0 = self.remainder.1 + 1;
            Some(result)
        }
    }
}
