
                node('something') {
                  stage('whatever') {
                    myLib.someFunction(
                     aoeu: 'foo',
                     htns: 'bar'
                     ) { blah ->
                    println(blah)
                    }
                  }
                }
            