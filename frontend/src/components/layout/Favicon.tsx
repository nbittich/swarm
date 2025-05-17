import React, { useRef, } from "react";
import "./Favicon.css";

type Props = {
    handleNavigation: (path: string) => void;
};

const FaviconIcon: React.FC<Props> = ({ handleNavigation }) => {
    const audioRef = useRef<HTMLAudioElement | null>(null);
    const imgRef = useRef<HTMLImageElement | null>(null);
    const handleRightClick = (e: React.MouseEvent<HTMLImageElement, MouseEvent>) => {
        e.preventDefault();

        if (audioRef.current) {
            audioRef.current.currentTime = 0;
            audioRef.current.play();

            if (imgRef.current) {
                imgRef.current.classList.remove("animate-loop");
                void imgRef.current.offsetWidth;
                imgRef.current.classList.add("animate-loop");
            }
        }
    };

    return (
        <>
            <a onClick={() => handleNavigation("/")}>
                <img
                    onContextMenu={handleRightClick}
                    className="favicon-icon"
                    ref={imgRef}
                    src="/favicon.svg"
                    width={24}
                    height={24}
                    style={{ verticalAlign: "middle" }}
                />
            </a>
            <audio ref={audioRef} src="/batman.mp3" />
        </>
    );
};

export default FaviconIcon;
