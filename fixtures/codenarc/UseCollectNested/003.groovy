
            class MyBuilder {
                void execute(Closure antClosure) {
                    Closure converter = {File file -> file.toURI().toURL() }
                    List<URL> classpathUrls = fullClasspath.collect(converter)
                }
            }
        