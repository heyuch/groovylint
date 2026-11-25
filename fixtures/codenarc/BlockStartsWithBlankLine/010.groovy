
            class KnownLimitationsClass {
                boolean oddlyFormattedMethod() { true

                }

                String startingWithNonEmptyLine() {
                    "value"
                }

                def methodWithClosures() {
                    def list = []
                    list.each { item
                        ->

                        1
                    }

                    list.each {
                        item ->

                    }

                    [:].each {
                        def key,
                        def value ->

                    }

                }
            }
        