
            job('test-job') {
                triggers { // fails here, no new line above
                    cron('* * * * *')
                }
            
                steps {
                    groovyScriptFile('my_script.groovy')
                }
            }
        