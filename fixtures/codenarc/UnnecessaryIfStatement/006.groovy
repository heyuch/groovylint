
            def isSpellingCorrect(word) {
                File file = new File("...")
                def found = false
                file.eachLine {
                    if (it == word) found = true
                }
                if (found) { true } else false
            }
        