
            class MyClass {
                def mapping = myService.findAllMappings(0)?.collect { domain ->
                  [description: countryCode?.padRight(6, '\u00A0')+ domain.countryName]
                }

                String myString = ready ? '\u00A0': 'error'+'99'
            }
        