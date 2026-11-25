
            enum Visibility {
                PUBLIC('public'), PROTECTED('protected'), PRIVATE('private')

                private final String name

                private Visibility(String name) {
                    this.name = name
                }

                String getName() {
                    return name
                }
            }
        