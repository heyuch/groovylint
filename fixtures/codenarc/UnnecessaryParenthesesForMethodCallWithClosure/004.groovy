
            dependencies {
                testCompile "org.springframework:spring-test:$springVersion",
                            ('org.spockframework:spock-core:0.5-groovy-1.8') {
                                exclude group: 'org.codehaus.groovy'
                            }
            }
        