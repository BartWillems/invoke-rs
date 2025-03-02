use std::fmt::Display;

use crate::utils::searxng::SearchResult;

pub fn summary(content: impl Display) -> String {
    format!(
        "Provide a concise summary of the provided text.
Provide the summary in bullet points when appropriate.

DO NOT repeat the question.
DO NOT include a reference section.
End with a 1 sentence summary of the text.

<text>
{content}
</text>"
    )
}

pub fn deep_search(question: impl Display, sources: Vec<SearchResult>) -> String {
    let formatted_sources = sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            format!(
                "Citation {}. Title: {}\nURL: {}\nSummary: {}",
                index + 1,
                source.title,
                source.url,
                source.content
            )
        })
        .collect::<Vec<String>>()
        .join("\n\n");

    format!("Generate a comprehensive and informative answer for a given question solely based on the provided web Search Results (URL, Page Title, Summary). You must only use information from the provided search results. Use an unbiased and journalistic tone.

You must cite the answer using [number] notation. You must cite sentences with their relevant citation number. Cite every part of the answer.
Place citations at the end of the sentence. You can do multiple citations in a row with the format [number1][number2].

Only cite the most relevant results that answer the question accurately. If different results refer to different entities with the same name, write separate answers for each entity.

When you include citations in your response, you absolutely have to list the sources at the bottom of your response with their URL in the following format:
- Citation [number]: [URL]
- Citation [number]: [URL]
- ...

ONLY cite inline.
DO NOT include a reference section, DO NOT include URLs.
DO NOT repeat the question.
DO NOT include citations WITHOUT referencing the search result's URL at the bottom.


You can use markdown formatting. You should include bullets to list the information in your answer.

<context>
{formatted_sources}
</context>
---------------------

Make sure to match the language of the user's question.

Question: {question}
Answer (in the language of the user's question): \
")
}
