
            def "plugin does not apply idea plugin"() {
                given:
                buildScript << """
                    task $testTaskName {
                        doLast {
                            println "Has idea plugin: \${project.plugins.hasPlugin(IdeaPlugin)}"
                        }
                    }
                """

                expect:
                runTask(testTaskName).output.contains('Has idea plugin: false')

                where:
                testTaskName = 'hasIdeaPlugin'
            }
            