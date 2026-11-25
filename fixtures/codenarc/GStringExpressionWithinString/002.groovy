
            @SomeAnnotationOnClass('${sample.property1}')
            class SomeClass {
                @SomeAnnotationOnField('${sample.property2}')
                String sampleProperty

                @SomeAnnotationOnMethod('${sample.property3}')
                void method() {
                }
            }
        