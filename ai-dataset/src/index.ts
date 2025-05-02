import Together from "together-ai";
import { createInterface } from 'readline/promises';
import token from "./token";

async function main() {
    process.env.TOGETHER_API_KEY = token;
    const together = new Together();

    const messages: {
        role: "system" | "user" | "assistant" | "tool";
        content: string;
    }[] = [
            { "role": "system", "content": "You are an expert in semantic data, SPARQL and RDF." },
        ];
    const rl = createInterface({
        input: process.stdin,
        output: process.stdout,
    });


    const message = `generate questions/answers (instruction /input output for ai training) based on the following data:
	ns77:fac4fba5-e3b7-4afb-aae9-908a688c597a	rdf:type	ns18:BesluitNieuweStijl ,
			ns19:e96ec8af-6480-4b32-876a-fefe5f0a3793 ,
			ns7:Besluit ;
		ns13:language	ns20:NLD ;
		prov:wasDerivedFrom	ns110:besluitenlijst ;
		ns13:title	"Debiteurenbeheer - rapportering financieel directeur. Kennisgeving."@nl ;
		ns13:description	"Het decreet over het lokaal bestuur schrijft voor dat de financieel directeur in volle onafhankelijkheid instaat voor het debiteurenbeheer van het lokaal bestuur. De gemeente en het OCMW doen heel wat ontvangsten. Vanaf het ogenblik dat ontvangsten niet contant worden ge\u00EFnd ontstaat er een vordering. Debiteurenbeheer is het verwerken en opvolgen van deze vorderingen vanaf het ontstaan ervan tot en met de inning (of het oninvorderbaar stellen).Voor de rapportering is ervoor geopteerd om voornamelijk in te zoomen op een aantal statistieken inzake de 4 grootste \u2013 jaarlijkse weerkerende - belastingen (AGB gezinnen, AGB bedrijven, tweede verblijven, Limburg.net). Voor niet-fiscale vorderingen wordt gerapporteerd over een aantal globale cijfers. De gepresenteerde cijfers zijn een momentopname, een foto op een bepaald tijdstip. De rapportering is opgenomen als bijlage bij dit besluit.Deze rapportering zal jaarlijks in het begin van het jaar een update krijgen."@nl ;
		ns6:uuid	"01967d8a3a337c22971efaa11b103b8b" .`;
    messages.push({ "role": "user", "content": message });
    const response = await together.chat.completions.create({
        messages,
        model: "meta-llama/Llama-3.3-70B-Instruct-Turbo-Free"
    });
    for (const choice of response!.choices) {
        console.log(choice.message!.content)
        messages.push({ "role": "assistant", "content": choice.message!.content || "" });
    }

    let prompt = await rl.question("");

    while (prompt != "exit") {

        messages.push({ "role": "user", "content": prompt });
        const response = await together.chat.completions.create({
            messages,
            model: "meta-llama/Llama-3.3-70B-Instruct-Turbo-Free"
        });
        for (const choice of response!.choices) {
            console.log(choice.message!.content)
            messages.push({ "role": "assistant", "content": choice.message!.content || "" });
        }
        prompt = await rl.question("");

    }
}

main().then()

