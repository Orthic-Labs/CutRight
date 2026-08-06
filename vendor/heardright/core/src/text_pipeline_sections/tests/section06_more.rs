    #[test]
    fn normalizes_dates_units_fillers_and_snippets() {
        assert_eq!(
            deterministic_polish("um meet me on november fifth"),
            "Meet me on November 5th"
        );
        assert_eq!(
            deterministic_polish("ship it by the twenty first of june 2026"),
            "Ship it by June 21, 2026"
        );
        assert_eq!(
            deterministic_polish("due on june twenty third twenty twenty six"),
            "Due on June 23, 2026"
        );
        assert_eq!(
            deterministic_polish("due on june twenty third two thousand twenty six"),
            "Due on June 23, 2026"
        );
        assert_eq!(
            deterministic_polish("record for five minutes and twenty megabytes"),
            "Record for 5 minutes and 20 MB"
        );
        let snippets = HashMap::from([
            ("email".to_string(), "adrian@example.com".to_string()),
            ("sig".to_string(), "Best,\nAdrian".to_string()),
        ]);

        assert_eq!(
            aggressive_speech_cleanup("So, okay, I mean we should ship it, you know, today"),
            "we should ship it today"
        );
        assert_eq!(
            aggressive_speech_cleanup("I like this kind of workflow"),
            "I like this kind of workflow"
        );
        assert_eq!(
            expand_snippets("send slash email then /sig", &snippets),
            "send adrian@example.com then Best,\nAdrian"
        );
    }

    #[test]
    fn normalizes_large_numbers_decimals_and_currency() {
        assert_eq!(
            deterministic_polish("about two million dollars"),
            "About $2,000,000"
        );
        assert_eq!(
            deterministic_polish("fifty thousand users signed up"),
            "50000 users signed up"
        );
        assert_eq!(
            deterministic_polish("revenue rose nine point four percent"),
            "Revenue rose 9.4%"
        );
        assert_eq!(
            deterministic_polish("pi is three point one four"),
            "Pi is 3.14"
        );
        assert_eq!(
            deterministic_polish("we lost two point five million dollars"),
            "We lost $2,500,000"
        );
    }

    #[test]
    fn full_passage_fragment_normalizes() {
        assert_eq!(
            deterministic_polish(
                "revenue rose nine point four percent to about two million dollars"
            ),
            "Revenue rose 9.4% to about $2,000,000"
        );
    }

    #[test]
    fn preserves_existing_small_number_behavior() {
        assert_eq!(deterministic_polish("twenty dollars"), "$20");
        assert_eq!(deterministic_polish("five percent"), "5%");
        assert_eq!(
            deterministic_polish("the room should be set to 22 degrees c with 50 percent humidity"),
            "The room should be set to 22°C with 50% humidity"
        );
        assert_eq!(
            deterministic_polish("please add the hashtag launch review"),
            "Please add the #launch review"
        );
        assert_eq!(
            deterministic_polish("one hundred sixty five commands"),
            "165 commands"
        );
    }

    #[test]
    fn fixes_punctuation_spacing_and_proper_nouns() {
        assert_eq!(
            deterministic_polish("adrian ,heard right and new york"),
            "Adrian, HeardRight and New York"
        );
        assert_eq!(
            deterministic_polish("dji mic mini and rtx 4070 use dml"),
            "DJI Mic Mini and RTX 4070 use DML"
        );
    }

    #[test]
    fn heardright_brand_casing_spares_the_verb_idiom() {
        // The brand: "heard right" preceded by punctuation/article/noun → HeardRight.
        assert_eq!(
            deterministic_polish("the heard right app"),
            "The HeardRight app"
        );
        assert_eq!(
            deterministic_polish("adrian ,heard right and new york"),
            "Adrian, HeardRight and New York"
        );
        // The idiom: "<pronoun> heard right" is the verb — must stay verbatim
        // (whisper_eval clip05 regression). The old unconditional rule corrupted these.
        assert_eq!(
            deterministic_polish("i heard right the first time"),
            "I heard right the first time"
        );
        assert_eq!(deterministic_polish("you heard right"), "You heard right");
    }

    #[test]
    fn casing_unambiguous_brands() {
        assert_eq!(
            deterministic_polish("we looked at the square space site"),
            "We looked at the Squarespace site"
        );
        // Client-specific names must never be introduced by the suite-wide
        // deterministic defaults. Users can add them through Vocabulary.
        assert!(!deterministic_polish(
            "the one click drive footer on youtube and tiktok"
        )
        .contains("OneClickDrive"));
        assert_eq!(
            deterministic_polish("adrian works with damned designs rotten hand and stunning strangers"),
            "Adrian works with damned designs rotten hand and stunning strangers"
        );
        assert_eq!(
            deterministic_polish("oracle netflix and huawei use instagram"),
            "Oracle Netflix and Huawei use Instagram"
        );
    }

    #[test]
    fn tail_polish_does_not_capitalize_fragment_start() {
        assert_eq!(deterministic_polish_tail("after lunch"), "after lunch");
        assert_eq!(
            deterministic_polish_tail("after lunch. send it"),
            "after lunch. Send it"
        );
    }
