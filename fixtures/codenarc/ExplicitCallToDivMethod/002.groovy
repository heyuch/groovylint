
            html {
                body {
                    div("Some text")
                    div(a:'aaa', b:'bbb')
                    div { p("some text") }
                    // div(7)           // violation
                    // div(3.1415)      // violation
                    //div(someVariable) // violation
                }
            }
        