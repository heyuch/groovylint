
            enum Visibility {
                PUBLIC('public'), PROTECTED('protected'), PRIVATE('private')
                final String name
                private Visibility(String name) {
                    this.name = name
                }
            }
        